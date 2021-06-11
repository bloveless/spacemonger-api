package test

import (
	"fmt"
	"net/http"
	"net/http/httptest"
	"spacemonger"
	"testing"
)

func TestGetServerStatus(t *testing.T) {
	ts := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		w.Header().Set("Content-Type", "application/json")
		fmt.Fprintln(w, `{"status": "Game is up and available to play"}`)
	}))
	defer ts.Close()

	c, err := spacemonger.NewClient()
	if err != nil {
		t.Fail()
	}

	c.SetBaseUrl(ts.URL)

	gs, err := c.GetGameStatus()
	if err != nil {
		t.Fatalf("Failed: getting game status %s\n", err)
	}

	if gs.Status != "Game is up and available to play" {
		t.Fatal("Returned the wrong value")
	}
}

func TestGetServerStatusError(t *testing.T) {
	ts := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		w.Header().Set("Content-Type", "application/json")
		w.WriteHeader(http.StatusBadRequest)
		fmt.Fprintln(w, `{"error": {"message": "An error occurred", "code": 40001}}`)
	}))
	defer ts.Close()

	c, err := spacemonger.NewClient()
	if err != nil {
		t.Fail()
	}

	c.SetBaseUrl(ts.URL)

	_, err = c.GetGameStatus()
	if err == nil {
		t.Fatalf("Expected a SpaceTraderError but the request succeeded")
	}
}
