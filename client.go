package spacemonger

import (
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
	httpMutex  *sync.Mutex
	baseUrl    string
	token      string
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
		baseUrl:   "https://api.spacetraders.io",
		httpMutex: &sync.Mutex{},
	}, nil
}

// SetBaseUrl will override the default base url of https://api.spacetraders.io. This is only used for testing.
func (c *Client) SetBaseUrl(base string) {
	c.baseUrl = base
}

// SetToken will set the token on the client for a specific user
func (c *Client) SetToken(token string) {
	c.token = token
}

func (c *Client) executeRequest(method string, url string, token string, body io.Reader, decodeResponse interface{}) error {
	fullUrl := url
	if !strings.Contains(fullUrl, "http://") && !strings.Contains(fullUrl, "https://") {
		fullUrl = c.baseUrl + url
	}

	request, err := http.NewRequest(method, fullUrl, body)
	if err != nil {
		return err
	}

	if token != "" {
		request.Header.Add("Authorization", fmt.Sprintf("Bearer %s", token))
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

		if response.StatusCode >= 200 && response.StatusCode < 300 {
			if err := json.Unmarshal(responseBody, decodeResponse); err != nil {
				return UnableToDecodeResponse
			}

			return nil
		}

		if response.StatusCode == 401 {
			return Unauthorized
		}

		// Now it is time for some error handling
		if response.StatusCode == 429 {
			retryAfter, err := strconv.ParseFloat(response.Header.Get("retry-after"), 64)
			if err != nil {
				return errors.New("unable to parse retry-after header as float64")
			}

			waitTime := time.Duration(retryAfter*1000) * time.Millisecond
			fmt.Printf("Rate limited... waiting for %v seconds before trying again. Request: \"%s %s\"\n", waitTime, method, url)

			time.Sleep(waitTime)
			continue
		}

		if response.StatusCode == 500 {
			// If there was an internal server error then try the request again in 2 seconds
			fmt.Printf("Caught internal server error retrying in 2 seconds. %s", responseBody)
			time.Sleep(2 * time.Second)

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

// GetMyIpAddress will get the clients current external ip address
func (c *Client) GetMyIpAddress() (MyIpAddressResponse, error) {
	r := MyIpAddressResponse{}
	err := c.executeRequest("GET", "https://api.ipify.org?format=json", "", nil, &r)
	if err != nil {
		return MyIpAddressResponse{}, err
	}

	return r, nil
}

// ClaimUsername will claim a username and return a token
func (c *Client) ClaimUsername(username string) (ClaimUsernameResponse, error) {
	r := ClaimUsernameResponse{}
	err := c.executeRequest("POST", fmt.Sprintf("/users/%s/token", username), "", nil, &r)
	if err != nil {
		return ClaimUsernameResponse{}, err
	}

	return r, nil
}

// GetGameStatus will return the current status of https://api.spacetraders.io
func (c *Client) GetGameStatus() (GameStatusResponse, error) {
	r := GameStatusResponse{}
	err := c.executeRequest("GET", "/game/status", "", nil, &r)
	if err != nil {
		return GameStatusResponse{}, err
	}

	return r, nil
}

// ////////////////////////////////////////////
// /// ACCOUNT
// ////////////////////////////////////////////

// GetMyInfo returns the current users info
func (c *Client) GetMyInfo() (UserInfo, error) {
	r := UserInfo{}
	err := c.executeRequest("GET", "https://api.spacetraders.io/my/account", c.token, nil, &r)
	if err != nil {
		return UserInfo{}, err
	}

	return r, nil
}

func (c *Client) GetFlightPlan(flightPlanId string) (FlightPlan, error) {
	r := FlightPlan{}
	err := c.executeRequest("GET", fmt.Sprintf("https://api.spacetraders.io/my/flight-plans/%s", flightPlanId), c.token, nil, &r)
	if err != nil {
		return FlightPlan{}, err
	}

	return r, nil
}
