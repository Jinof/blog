package main

import (
	"fmt"
	"os"
)

func main() {
	root, err := os.Getwd()
	if err == nil {
		err = run(root, os.Args[1:])
	}
	if err != nil {
		fmt.Fprintln(os.Stderr, "error:", err)
		os.Exit(1)
	}
}

func run(root string, args []string) error {
	command := "build"
	if len(args) > 0 {
		command = args[0]
	}
	switch command {
	case "build":
		return buildSite(root, includeDraftPosts(args))
	case "serve":
		return serveSite(root, args)
	default:
		return fmt.Errorf("unknown command: %s. Use `build` or `serve`.", command)
	}
}

func includeDraftPosts(args []string) bool {
	for _, arg := range args {
		if arg == "--draft" {
			return true
		}
	}
	return false
}

func previewAddress(args []string) string {
	port := "1313"
	for i := 0; i+1 < len(args); i++ {
		if args[i] == "--port" {
			port = args[i+1]
			break
		}
	}
	return "127.0.0.1:" + port
}
