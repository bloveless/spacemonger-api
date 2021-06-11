package spacemonger

import (
	"bytes"
	"encoding/json"
	"fmt"
	"io"
	"io/ioutil"
	"net/http"
	"net/url"
	"os"
	"time"
)

type Client struct {
	httpClient *http.Client
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
		baseUrl: "https://api.spacetraders.com",
	}, nil
}

func (c *Client) SetBaseUrl(base string) {
	c.baseUrl = base
}

func (c *Client) executeRequest(method string, url string, body io.Reader, decodeResponse interface{}) (interface{}, error) {
	request, err := http.NewRequest(method, c.baseUrl+url, body)
	if err != nil {
		return nil, err
	}

	response, err := c.httpClient.Do(request)
	if err != nil {
		// TODO: This error is an error executing the request and this probably an appropriate error to return
		return nil, err
	}

	defer response.Body.Close()

	responseBody, err := ioutil.ReadAll(response.Body)
	if err != nil {
		return nil, err
	}

	// TODO: We should check the status codes here for errors and parse into the
	//       spacetraders error struct here if that is the case and return that error here

	if response.StatusCode >= 200 && response.StatusCode < 300 {
		dec := json.NewDecoder(bytes.NewReader(responseBody))
		dec.DisallowUnknownFields() // Force errors

		if err := dec.Decode(decodeResponse); err != nil {
			// We were unable to decode the response
			return nil, err
		}

		return decodeResponse, nil
	}

	// Now it is time for some error handling
	if response.StatusCode == 429 {
		// TODO: Handle ratelimiting retry logic
	}

	e := &SpaceTraderError{}
	err = json.Unmarshal(responseBody, &e)
	if err != nil {
		return nil, err
	}

	return nil, e
}

func (c *Client) GetMyIpAddress() (*MyIpAddressResponse, error) {
	r, err := c.executeRequest("GET", "https://api.ipify.org?format=json", nil, &MyIpAddressResponse{})
	if err != nil {
		return nil, err
	}

	return r.(*MyIpAddressResponse), nil
}

func (c *Client) GetGameStatus() (*GameStatusResponse, error) {
	r, err := c.executeRequest("GET", "/game/status", nil, &GameStatusResponse{})
	if err != nil {
		return nil, err
	}

	return r.(*GameStatusResponse), nil
}

func (c *Client) ClaimUsername(username string) (*ClaimUsernameResponse, error) {
	r, err := c.executeRequest("POST", fmt.Sprintf("/users/%s/token", username), nil, &ClaimUsernameResponse{})
	if err != nil {
		return nil, err
	}

	return r.(*ClaimUsernameResponse), nil
}
