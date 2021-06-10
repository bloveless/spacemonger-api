package spacemonger

import (
	"encoding/json"
	"io"
	"io/ioutil"
	"net/http"
	"net/url"
	"os"
	"time"
)

type Client struct {
	httpClient *http.Client
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
	}, nil
}

func (c *Client) executeRequest(method string, url string, body io.Reader) ([]byte, error) {
	request, err := http.NewRequest(method, url, body)
	if err != nil {
		return []byte{}, err
	}

	response, err := c.httpClient.Do(request)
	if err != nil {
		// TODO: This error is an error executing the request and this probably an appropriate error to return
		return []byte{}, err
	}

	defer response.Body.Close()

	responseBody, err := ioutil.ReadAll(response.Body)
	if err != nil {
		return []byte{}, err
	}

	// TODO: We should check the status codes here for errors and parse into the
	// 		 spacetraders error struct here if that is the case and return that error here

	return responseBody, err
}

func (c *Client) GetMyIpAddress() (*MyIpAddress, error) {
	response, err := c.executeRequest("GET", "https://api.ipify.org?format=json", nil)
	if err != nil {
		return &MyIpAddress{}, err
	}

	r := &MyIpAddress{}
	err = json.Unmarshal(response, r)
	if err != nil {
		return &MyIpAddress{}, err
	}

	return r, nil
}

func (c *Client) GetGameStatus() (*GameStatus, error) {
	response, err := c.executeRequest("GET", "https://api.spacetraders.io/game/status", nil)
	if err != nil {
		return &GameStatus{}, err
	}

	r := &GameStatus{}
	err = json.Unmarshal(response, r)
	if err != nil {
		return &GameStatus{}, err
	}

	return r, nil
}
