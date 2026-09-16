package main

import (
	"context"
	"crypto/sha256"
	"fmt"
	"io/fs"
	"log"
	"net"
	"net/http"
	"os"
	"path/filepath"
	"strconv"
	"strings"
	"sync"
	"time"
)

type preview struct {
	root          string
	includeDrafts bool
	mu            sync.RWMutex
	previous      string
}

func newPreview(root string, includeDrafts bool) (*preview, error) {
	// Capture before building: a change during the build must trigger another build.
	signature, err := sourceSignature(root)
	if err != nil {
		return nil, err
	}
	if err := buildSite(root, includeDrafts); err != nil {
		return nil, err
	}
	return &preview{root: root, includeDrafts: includeDrafts, previous: signature}, nil
}

func serveSite(root string, args []string) error {
	p, err := newPreview(root, includeDraftPosts(args))
	if err != nil {
		return err
	}
	address := previewAddress(args)
	listener, err := net.Listen("tcp", address)
	if err != nil {
		return fmt.Errorf("bind %s: %w", address, err)
	}
	ctx, cancel := context.WithCancel(context.Background())
	defer cancel()
	go p.poll(ctx, time.Second)
	fmt.Printf("Serving http://%s/\n", listener.Addr())
	server := &http.Server{Handler: p, ReadTimeout: 5 * time.Second, WriteTimeout: 30 * time.Second}
	return server.Serve(listener)
}

func (p *preview) rebuildIfChanged() error {
	p.mu.Lock()
	defer p.mu.Unlock()
	current, err := sourceSignature(p.root)
	if err != nil {
		return err
	}
	if current == p.previous {
		return nil
	}
	if err := buildSite(p.root, p.includeDrafts); err != nil {
		return err
	}
	p.previous = current
	return nil
}

func (p *preview) poll(ctx context.Context, interval time.Duration) {
	ticker := time.NewTicker(interval)
	defer ticker.Stop()
	for {
		select {
		case <-ctx.Done():
			return
		case <-ticker.C:
			if err := p.rebuildIfChanged(); err != nil {
				log.Printf("rebuild error: %v", err)
			}
		}
	}
}

func (p *preview) ServeHTTP(w http.ResponseWriter, r *http.Request) {
	p.mu.RLock()
	file := resolvePublicPath(filepath.Join(p.root, "public"), r.URL.Path)
	contents, err := os.ReadFile(file)
	p.mu.RUnlock()
	if err != nil {
		w.Header().Set("Content-Type", "text/plain; charset=utf-8")
		w.Header().Set("Content-Length", "9")
		w.WriteHeader(http.StatusNotFound)
		if r.Method != http.MethodHead {
			_, _ = w.Write([]byte("Not found"))
		}
		return
	}
	w.Header().Set("Content-Type", contentType(file))
	w.Header().Set("Content-Length", strconv.Itoa(len(contents)))
	if r.Method != http.MethodHead {
		_, _ = w.Write(contents)
	}
}

// net/http has already percent-decoded URL.Path; never decode it a second time.
func resolvePublicPath(publicDir, route string) string {
	path := publicDir
	for _, part := range strings.Split(strings.TrimLeft(route, "/"), "/") {
		if part == ".." || part == "" {
			continue
		}
		path = filepath.Join(path, part)
	}
	if strings.HasSuffix(route, "/") || filepath.Ext(path) == "" {
		path = filepath.Join(path, "index.html")
	}
	return path
}

func sourceSignature(root string) (string, error) {
	hash := sha256.New()
	for _, directory := range []string{"posts", "assets"} {
		source := filepath.Join(root, directory)
		err := filepath.WalkDir(source, func(path string, entry fs.DirEntry, err error) error {
			if os.IsNotExist(err) && path == source {
				return nil
			}
			if err != nil {
				return err
			}
			if entry.IsDir() {
				return nil
			}
			info, err := entry.Info()
			if err != nil {
				return err
			}
			relative, err := filepath.Rel(root, path)
			if err != nil {
				return err
			}
			fmt.Fprintf(hash, "%s:%d:%d\n", relative, info.ModTime().UnixNano(), info.Size())
			return nil
		})
		if err != nil {
			return "", err
		}
	}
	config, err := os.ReadFile(filepath.Join(root, "site.config.json"))
	if err != nil {
		return "", fmt.Errorf("read site.config.json: %w", err)
	}
	_, _ = hash.Write(config)
	return fmt.Sprintf("%x", hash.Sum(nil)), nil
}

func contentType(file string) string {
	switch filepath.Ext(file) {
	case ".css":
		return "text/css; charset=utf-8"
	case ".gif":
		return "image/gif"
	case ".html":
		return "text/html; charset=utf-8"
	case ".ico":
		return "image/x-icon"
	case ".jpg", ".jpeg":
		return "image/jpeg"
	case ".js":
		return "text/javascript; charset=utf-8"
	case ".png":
		return "image/png"
	case ".svg":
		return "image/svg+xml"
	case ".txt":
		return "text/plain; charset=utf-8"
	case ".wasm":
		return "application/wasm"
	case ".webp":
		return "image/webp"
	case ".xml":
		return "application/xml; charset=utf-8"
	default:
		return "application/octet-stream"
	}
}
