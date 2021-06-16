package test

import (
	"errors"
	"fmt"
	"net/http"
	"net/http/httptest"
	"spacemonger/spacetrader"
	"testing"
)

func TestInvalidJsonResponse(t *testing.T) {
	ts := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		w.Header().Set("Content-Type", "application/json")
		w.WriteHeader(http.StatusOK)
		fmt.Fprintln(w, `{this is invalid json}`)
	}))
	defer ts.Close()

	c, err := spacetrader.NewClient()
	if err != nil {
		t.Fail()
		return
	}

	c.SetBaseUrl(ts.URL)

	_, err = c.GetGameStatus()
	if !errors.Is(err, spacetrader.UnableToDecodeResponse) {
		t.Fatalf("Expected an UnableToDecodeResponse error")
	}
}

func TestReceiveSpaceTraderApiError(t *testing.T) {
	ts := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		w.Header().Set("Content-Type", "application/json")
		w.WriteHeader(http.StatusBadRequest)
		fmt.Fprintln(w, `{"error": {"message": "An error occurred", "code": 40001}}`)
	}))
	defer ts.Close()

	c, err := spacetrader.NewClient()
	if err != nil {
		t.Fail()
		return
	}

	c.SetBaseUrl(ts.URL)

	_, err = c.GetGameStatus()
	if err == nil {
		t.Fatalf("Expected a SpaceTraderError but the request succeeded")
	}
}

func TestRetryRateLimitFailThenSucceed(t *testing.T) {
	attemptCount := 0
	ts := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		if attemptCount == 0 {
			attemptCount += 1
			w.Header().Set("Content-Type", "application/json")
			w.Header().Set("retry-after", "0.005")
			w.WriteHeader(http.StatusTooManyRequests)
			fmt.Fprintln(w, `{"error": {"message": "Too many requests", "code": 42901}}`)
		} else {
			w.Header().Set("Content-Type", "application/json")
			w.WriteHeader(http.StatusOK)
			fmt.Fprintln(w, `{"status": "Game is up and available to play"}`)
		}
	}))
	defer ts.Close()

	c, err := spacetrader.NewClient()
	if err != nil {
		t.Fail()
		return
	}

	c.SetBaseUrl(ts.URL)

	_, err = c.GetGameStatus()
	if err != nil {
		t.Fatalf("Expected game status request to have succeeded after one retry")
	}
}

func TestRetryRateLimitAlwaysFail(t *testing.T) {
	ts := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		w.Header().Set("Content-Type", "application/json")
		w.Header().Set("retry-after", "0.005")
		w.WriteHeader(http.StatusTooManyRequests)
		fmt.Fprintln(w, `{"error": {"message": "Too many requests", "code": 42901}}`)
	}))
	defer ts.Close()

	c, err := spacetrader.NewClient()
	if err != nil {
		t.Fail()
		return
	}

	c.SetBaseUrl(ts.URL)

	_, err = c.GetGameStatus()
	if err == nil {
		t.Fatalf("Expected game status request to have retried three times and then failed")
	}
}

func TestInternalServerFailureThenSucceed(t *testing.T) {
	attemptCount := 0
	ts := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		if attemptCount == 0 {
			attemptCount += 1
			w.Header().Set("Content-Type", "application/json")
			w.Header().Set("retry-after", "0.005")
			w.WriteHeader(http.StatusInternalServerError)
			fmt.Fprintln(w, `{"error": {"message": "Too many requests", "code": 42901}}`)
		} else {
			w.Header().Set("Content-Type", "application/json")
			w.WriteHeader(http.StatusOK)
			fmt.Fprintln(w, `{"status": "Game is up and available to play"}`)
		}
	}))
	defer ts.Close()

	c, err := spacetrader.NewClient()
	if err != nil {
		t.Fail()
		return
	}

	c.SetBaseUrl(ts.URL)

	_, err = c.GetGameStatus()
	if err != nil {
		t.Fatalf("Expected game status request to have succeeded after one retry")
	}
}

func TestInternalServerFailureAlwaysFail(t *testing.T) {
	ts := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		w.Header().Set("Content-Type", "application/json")
		w.WriteHeader(http.StatusInternalServerError)
		fmt.Fprintln(w, `{"error": {"message": "Too many requests", "code": 42901}}`)
	}))
	defer ts.Close()

	c, err := spacetrader.NewClient()
	if err != nil {
		t.Fail()
		return
	}

	c.SetBaseUrl(ts.URL)

	_, err = c.GetGameStatus()
	if !errors.Is(err, spacetrader.TooManyRetries) {
		t.Fatalf("Expected request to be retried and then fail with TooManyRetries")
	}
}

func TestGetServerStatus(t *testing.T) {
	ts := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		w.Header().Set("Content-Type", "application/json")
		w.WriteHeader(http.StatusOK)
		fmt.Fprintln(w, `{"status": "Game is up and available to play"}`)
	}))
	defer ts.Close()

	c, err := spacetrader.NewClient()
	if err != nil {
		t.Fail()
		return
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
