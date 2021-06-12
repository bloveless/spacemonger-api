package spacemonger

import (
	"bytes"
	"encoding/json"
	"errors"
	"fmt"
	"io"
	"io/ioutil"
	"net/http"
	"net/url"
	"os"
	"strconv"
	"strings"
	"sync"
	"time"
)

type Client struct {
	httpClient *http.Client
	httpMutex *sync.Mutex
	baseUrl    string
}

func NewClient() (*Client, error) {
	transport := &http.Transport{}

	envProxy := os.Getenv("HTTP_PROXY")
	if envProxy != "" {
		proxy, err := url.Parse(envProxy)
		if err != nil {
			return nil, err
		}
		transport.Proxy = http.ProxyURL(proxy)
	}

	return &Client{
		httpClient: &http.Client{
			Transport: transport,
			Timeout:   time.Second * 10,
		},
		baseUrl: "https://api.spacetraders.io",
		httpMutex: &sync.Mutex{},
	}, nil
}

func (c *Client) SetBaseUrl(base string) {
	c.baseUrl = base
}

func (c *Client) executeRequest(method string, url string, body io.Reader, decodeResponse interface{}) error {
	fullUrl := url
	if !strings.Contains(fullUrl, "http://") && !strings.Contains(fullUrl, "https://") {
		fullUrl = c.baseUrl+url
	}

	request, err := http.NewRequest(method, fullUrl, body)
	if err != nil {
		return err
	}

	attemptCount := 0
	for {
		attemptCount += 1

		if attemptCount > 3 {
			return TooManyRetries
		}

		response, err := c.httpClient.Do(request)
		if err != nil {
			return err
		}

		responseBody, err := ioutil.ReadAll(response.Body)
		response.Body.Close()
		if err != nil {
			return err
		}

		// TODO: We should check the status codes here for errors and parse into the
		//       spacetraders error struct here if that is the case and return that error here

		if response.StatusCode >= 200 && response.StatusCode < 300 {
			dec := json.NewDecoder(bytes.NewReader(responseBody))
			dec.DisallowUnknownFields() // Force errors

			if err := dec.Decode(decodeResponse); err != nil {
				// We were unable to decode the response
				return err
			}

			return nil
		}

		// Now it is time for some error handling
		if response.StatusCode == 429 {
			// TODO: Handle rate limiting retry logic
			retryAfter, err := strconv.ParseFloat(response.Header.Get("retry-after"), 64)
			if err != nil {
				return errors.New("unable to parse retry-after header as float64")
			}

			waitTime := time.Duration(retryAfter*1000) * time.Millisecond
			fmt.Printf("Rate limited... waiting for %v seconds before trying again. Request: \"%s %s\"\n", waitTime, method, url);

			time.Sleep(waitTime)
			continue
		}

		e := &SpaceTraderError{}
		err = json.Unmarshal(responseBody, &e)
		if err != nil {
			return err
		}

		return e
	}
}

func (c *Client) GetMyIpAddress() (MyIpAddressResponse, error) {
	r := MyIpAddressResponse{}
	err := c.executeRequest("GET", "https://api.ipify.org?format=json", nil, &r)
	if err != nil {
		return MyIpAddressResponse{}, err
	}

	return r, nil
}

func (c *Client) GetGameStatus() (GameStatusResponse, error) {
	r := GameStatusResponse{}
	err := c.executeRequest("GET", "/game/status", nil, &r)
	if err != nil {
		return GameStatusResponse{}, err
	}

	return r, nil
}

func (c *Client) ClaimUsername(username string) (ClaimUsernameResponse, error) {
	r := ClaimUsernameResponse{}
	err := c.executeRequest("POST", fmt.Sprintf("/users/%s/token", username), nil, &r)
	if err != nil {
		return ClaimUsernameResponse{}, err
	}

	return r, nil
}
