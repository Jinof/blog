package main

import (
	"crypto/sha256"
	"encoding/hex"
	"encoding/json"
	"net/http"
	"net/http/httptest"
	"strings"
	"testing"
)

func TestUploadUnmarshalsAndHashesPayload(t *testing.T) {
	body := `{"payload":"xxxx"}`
	request := httptest.NewRequest(http.MethodPost, "/upload", strings.NewReader(body))
	request.Header.Set("Content-Type", "application/json")
	responseRecorder := httptest.NewRecorder()

	newRouter().ServeHTTP(responseRecorder, request)

	if responseRecorder.Code != http.StatusOK {
		t.Fatalf("status = %d, body = %s", responseRecorder.Code, responseRecorder.Body.String())
	}
	var response uploadResponse
	if err := json.Unmarshal(responseRecorder.Body.Bytes(), &response); err != nil {
		t.Fatalf("decode response: %v", err)
	}
	if response.ReceivedBytes != len(body) {
		t.Fatalf("received bytes = %d, want %d", response.ReceivedBytes, len(body))
	}
	if response.PayloadBytes != 4 {
		t.Fatalf("payload bytes = %d, want 4", response.PayloadBytes)
	}
	digest := sha256.Sum256([]byte("xxxx"))
	if response.SHA256 != hex.EncodeToString(digest[:]) {
		t.Fatalf("sha256 = %q", response.SHA256)
	}
}

func TestUploadRejectsInvalidJSON(t *testing.T) {
	request := httptest.NewRequest(http.MethodPost, "/upload", strings.NewReader(`{"payload":`))
	request.Header.Set("Content-Type", "application/json")
	responseRecorder := httptest.NewRecorder()

	newRouter().ServeHTTP(responseRecorder, request)

	if responseRecorder.Code != http.StatusBadRequest {
		t.Fatalf("status = %d, want %d", responseRecorder.Code, http.StatusBadRequest)
	}
}
