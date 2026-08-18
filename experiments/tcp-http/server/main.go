package main

import (
	"crypto/sha256"
	"encoding/hex"
	"encoding/json"
	"errors"
	"fmt"
	"io"
	"log"
	"net/http"
	"time"

	"github.com/gin-gonic/gin"
)

const maxBodyBytes = 80 * 1024 * 1024

type uploadRequest struct {
	Payload string `json:"payload"`
}

type uploadResponse struct {
	ReceivedBytes int     `json:"received_bytes"`
	PayloadBytes  int     `json:"payload_bytes"`
	ReadMS        float64 `json:"read_ms"`
	UnmarshalMS   float64 `json:"unmarshal_ms"`
	SHA256MS      float64 `json:"sha256_ms"`
	WorkMS        float64 `json:"work_ms"`
	SHA256        string  `json:"sha256"`
}

func milliseconds(duration time.Duration) float64 {
	return float64(duration.Microseconds()) / 1000
}

func newRouter() *gin.Engine {
	gin.SetMode(gin.ReleaseMode)
	router := gin.New()
	router.Use(gin.Recovery())
	if err := router.SetTrustedProxies(nil); err != nil {
		panic(err)
	}

	router.GET("/healthz", func(context *gin.Context) {
		context.String(http.StatusOK, "ok\n")
	})
	router.POST("/upload", uploadHandler)
	return router
}

func uploadHandler(context *gin.Context) {
	workStarted := time.Now()
	context.Request.Body = http.MaxBytesReader(
		context.Writer,
		context.Request.Body,
		maxBodyBytes,
	)

	readStarted := time.Now()
	body, err := io.ReadAll(context.Request.Body)
	readDuration := time.Since(readStarted)
	if err != nil {
		var tooLarge *http.MaxBytesError
		if errors.As(err, &tooLarge) {
			context.JSON(http.StatusRequestEntityTooLarge, gin.H{"error": "request body exceeds 80 MiB"})
			return
		}
		context.JSON(http.StatusBadRequest, gin.H{"error": "could not read request body"})
		return
	}

	unmarshalStarted := time.Now()
	var request uploadRequest
	if err := json.Unmarshal(body, &request); err != nil {
		context.JSON(http.StatusBadRequest, gin.H{"error": "request body is not valid upload JSON"})
		return
	}
	unmarshalDuration := time.Since(unmarshalStarted)
	if request.Payload == "" {
		context.JSON(http.StatusBadRequest, gin.H{"error": "payload must not be empty"})
		return
	}

	hashStarted := time.Now()
	digest := sha256.Sum256([]byte(request.Payload))
	hashDuration := time.Since(hashStarted)
	workDuration := time.Since(workStarted)
	response := uploadResponse{
		ReceivedBytes: len(body),
		PayloadBytes:  len(request.Payload),
		ReadMS:        milliseconds(readDuration),
		UnmarshalMS:   milliseconds(unmarshalDuration),
		SHA256MS:      milliseconds(hashDuration),
		WorkMS:        milliseconds(workDuration),
		SHA256:        hex.EncodeToString(digest[:]),
	}

	context.Header("X-Lab-Received-Bytes", fmt.Sprint(response.ReceivedBytes))
	context.Header("X-Lab-Unmarshal-Ms", fmt.Sprintf("%.3f", response.UnmarshalMS))
	context.Header("X-Lab-Work-Ms", fmt.Sprintf("%.3f", response.WorkMS))
	context.JSON(http.StatusOK, response)
	log.Printf(
		`{"event":"upload","client":%q,"received_bytes":%d,"payload_bytes":%d,"unmarshal_ms":%.3f,"sha256_ms":%.3f,"work_ms":%.3f}`,
		context.ClientIP(),
		response.ReceivedBytes,
		response.PayloadBytes,
		response.UnmarshalMS,
		response.SHA256MS,
		response.WorkMS,
	)
}

func main() {
	log.SetFlags(0)
	server := &http.Server{
		Addr:              ":8080",
		Handler:           newRouter(),
		ReadHeaderTimeout: 5 * time.Second,
		ReadTimeout:       2 * time.Minute,
		WriteTimeout:      15 * time.Second,
		IdleTimeout:       30 * time.Second,
	}
	log.Printf(`{"event":"listening","address":%q}`, server.Addr)
	if err := server.ListenAndServe(); err != nil && !errors.Is(err, http.ErrServerClosed) {
		log.Fatal(err)
	}
}
