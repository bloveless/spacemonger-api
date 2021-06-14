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

func (c *Client) executeRequest(method string, url string, body io.Reader, decodeResponse interface{}) error {
	fullUrl := url
	if !strings.Contains(fullUrl, "http://") && !strings.Contains(fullUrl, "https://") {
		fullUrl = c.baseUrl + url
	}

	request, err := http.NewRequest(method, fullUrl, body)
	if err != nil {
		return err
	}

	request.Header.Add("Content-Type", "application/json")
	if c.token != "" {
		request.Header.Add("Authorization", fmt.Sprintf("Bearer %s", c.token))
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
func (c *Client) GetMyIpAddress() (GetMyIpAddressResponse, error) {
	response := GetMyIpAddressResponse{}
	err := c.executeRequest("GET", "https://api.ipify.org?format=json", nil, &response)
	if err != nil {
		return GetMyIpAddressResponse{}, err
	}

	return response, nil
}

// ClaimUsername will claim a username and return a token
func (c *Client) ClaimUsername(username string) (ClaimUsernameResponse, error) {
	response := ClaimUsernameResponse{}
	err := c.executeRequest("POST", fmt.Sprintf("/users/%s/token", username), nil, &response)
	if err != nil {
		return ClaimUsernameResponse{}, err
	}

	return response, nil
}

// GetGameStatus will return the current status of https://api.spacetraders.io
func (c *Client) GetGameStatus() (GameStatusResponse, error) {
	response := GameStatusResponse{}
	err := c.executeRequest("GET", "/game/status", nil, &response)
	if err != nil {
		return GameStatusResponse{}, err
	}

	return response, nil
}

// ////////////////////////////////////////////
// /// ACCOUNT
// ////////////////////////////////////////////

// GetMyInfo returns the current users info
func (c *Client) GetMyInfo() (GetMyInfoResponse, error) {
	r := GetMyInfoResponse{}
	err := c.executeRequest("GET", "/my/account", nil, &r)
	if err != nil {
		return GetMyInfoResponse{}, err
	}

	return r, nil
}

// ////////////////////////////////////////////
// /// FLIGHT PLANS
// ////////////////////////////////////////////

func (c *Client) GetFlightPlan(flightPlanId string) (GetFlightPlanResponse, error) {
	response := GetFlightPlanResponse{}
	err := c.executeRequest("GET", fmt.Sprintf("/my/flight-plans/%s", flightPlanId), nil, &response)

	if err != nil {
		return GetFlightPlanResponse{}, err
	}

	return response, nil
}

func (c *Client) CreateFlightPlan(shipId, destination string) (CreateFlightPlanResponse, error) {
	request := CreateFlightPlanRequest{
		ShipId:      shipId,
		Destination: destination,
	}
	requestJson, err := json.Marshal(request)
	if err != nil {
		return CreateFlightPlanResponse{}, InvalidRequest
	}

	response := CreateFlightPlanResponse{}
	err = c.executeRequest("POST", "/my/flight-plans", bytes.NewReader(requestJson), &response)
	if err != nil {
		return CreateFlightPlanResponse{}, err
	}

	return response, nil
}

// ////////////////////////////////////////////
// /// LEADERBOARD
// ////////////////////////////////////////////

// TODO: leaderboard/networth

// ////////////////////////////////////////////
// /// LOANS
// ////////////////////////////////////////////

func (c *Client) GetMyLoans() (GetMyLoansResponse, error) {
	response := GetMyLoansResponse{}
	err := c.executeRequest("GET", "/my/loans", nil, &response)
	if err != nil {
		return GetMyLoansResponse{}, err
	}

	return response, nil
}

func (c *Client) PayOffLoan(loanId string) (PayOffLoanResponse, error) {
	response := PayOffLoanResponse{}
	// TODO: If this request doesn't work then it likely needs a body of any valid json payload
	err := c.executeRequest("PUT", fmt.Sprintf("/my/loans/%s", loanId), nil, &response)
	if err != nil {
		return PayOffLoanResponse{}, err
	}

	return response, nil
}

func (c *Client) CreateLoan(loanType string) (CreateLoanResponse, error) {
	request := CreateLoanRequest{
		LoanType: loanType,
	}
	requestJson, err := json.Marshal(request)
	if err != nil {
		return CreateLoanResponse{}, err
	}

	fmt.Printf("requestJson %+v\n", string(requestJson))

	response := CreateLoanResponse{}
	err = c.executeRequest("POST", "/my/loans", bytes.NewReader(requestJson), &response)
	if err != nil {
		return CreateLoanResponse{}, err
	}

	return response, nil
}

// ////////////////////////////////////////////
// /// LOCATIONS
// ////////////////////////////////////////////

func (c *Client) GetLocation(location string) (GetLocationResponse, error) {
	response := GetLocationResponse{}
	err := c.executeRequest("GET", fmt.Sprintf("/locations/%s", location), nil, &response)
	if err != nil {
		return GetLocationResponse{}, err
	}

	return response, nil
}

func (c *Client) GetLocationMarketplace(location string) (GetLocationMarketplaceResponse, error) {
	response := GetLocationMarketplaceResponse{}
	err := c.executeRequest("GET", fmt.Sprintf("locations/%s/marketplace", location), nil, &response)
	if err != nil {
		return GetLocationMarketplaceResponse{}, err
	}

	return response, nil
}

// TODO: Get Ships at a location

// ////////////////////////////////////////////
// /// PURCHASE ORDERS
// ////////////////////////////////////////////

func (c *Client) CreatePurchaseOrder(shipId, good string, quantity int) (CreatePurchaseOrderResponse, error) {
	request := CreatePurchaseOrderRequest{
		ShipId:   shipId,
		Good:     good,
		Quantity: quantity,
	}
	requestJson, err := json.Marshal(request)
	if err != nil {
		return CreatePurchaseOrderResponse{}, err
	}

	response := CreatePurchaseOrderResponse{}
	err = c.executeRequest("POST", "my/purchase-orders", bytes.NewReader(requestJson), &response)
	if err != nil {
		return CreatePurchaseOrderResponse{}, err
	}

	return response, nil
}

// ////////////////////////////////////////////
// /// SELL ORDERS
// ////////////////////////////////////////////

func (c *Client) CreateSellOrder(shipId, good string, quantity int) (CreateSellOrderResponse, error) {
	request := CreateSellOrderRequest{
		ShipId:   shipId,
		Good:     good,
		Quantity: quantity,
	}
	requestJson, err := json.Marshal(request)
	if err != nil {
		return CreateSellOrderResponse{}, err
	}

	response := CreateSellOrderResponse{}
	err = c.executeRequest("POST", "/my/sell-orders", bytes.NewReader(requestJson), &response)
	if err != nil {
		return CreateSellOrderResponse{}, err
	}

	return response, nil
}

// ////////////////////////////////////////////
// /// SHIPS
// ////////////////////////////////////////////

func (c *Client) PurchaseShip(location, shipType string) (PurchaseShipResponse, error) {
	request := PurchaseShipRequest{
		Location: location,
		ShipType: shipType,
	}
	requestJson, err := json.Marshal(request)
	if err != nil {
		return PurchaseShipResponse{}, nil
	}

	response := PurchaseShipResponse{}
	err = c.executeRequest("POST", "/my/ships", bytes.NewReader(requestJson), &response)
	if err != nil {
		return PurchaseShipResponse{}, nil
	}

	return response, nil
}

func (c *Client) GetMyShip(shipId string) (GetMyShipRequest, error) {
	response := GetMyShipRequest{}
	err := c.executeRequest("GET", fmt.Sprintf("/my/ships/%s", shipId), nil, &response)
	if err != nil {
		return GetMyShipRequest{}, nil
	}

	return response, nil
}

func (c *Client) GetMyShips() (GetMyShipsResponse, error) {
	response := GetMyShipsResponse{}
	err := c.executeRequest("GET", "/my/ships", nil, &response)
	if err != nil {
		return GetMyShipsResponse{}, nil
	}

	return response, nil
}

func (c *Client) JettisonCargo(shipId string, good string, quantity int) (JettisonCargoResponse, error) {
	request := JettisonCargoRequest{
		Good:     good,
		Quantity: quantity,
	}
	requestJson, err := json.Marshal(request)
	if err != nil {
		return JettisonCargoResponse{}, nil
	}

	response := JettisonCargoResponse{}
	err = c.executeRequest("POST", fmt.Sprintf("/my/ships/%s/jettison", shipId), bytes.NewReader(requestJson), &response)
	if err != nil {
		return JettisonCargoResponse{}, err
	}

	return response, nil
}

// TODO: Scrap your ship for credits
// TODO: Transfer cargo between ships

// ////////////////////////////////////////////
// /// STRUCTURES
// ////////////////////////////////////////////

// TODO: Create a new structure
// TODO: Deposit goods to a structure you own
// TODO: Deposit goods to a structure
// TODO: See specific structure
// TODO: Transfer goods from your structure to a ship
// TODO: Use to see a specific structure
// TODO: Use to see all of your structures

// ////////////////////////////////////////////
// /// SYSTEMS
// ////////////////////////////////////////////

func (c *Client) GetShipForSale() (GetShipsForSaleResponse, error) {
	response := GetShipsForSaleResponse{}
	err := c.executeRequest("GET", "/game/ships", nil, &response)
	if err != nil {
		return GetShipsForSaleResponse{}, nil
	}

	return response, nil
}

// TODO: Get all active flight plans in the system.
// TODO: Get info on a system's docked ships
// TODO: Get location info for a system
// TODO: Get systems info

func (c *Client) GetSystemsInfo() (GetSystemsInfoResponse, error) {
	response := GetSystemsInfoResponse{}
	err := c.executeRequest("GET", "/game/systems", nil, &response)
	if err != nil {
		return GetSystemsInfoResponse{}, nil
	}

	return response, nil
}

// ////////////////////////////////////////////
// /// TYPES
// ////////////////////////////////////////////

// TODO: Get available goods

func (c *Client) GetAvailableLoans() (GetAvailableLoansResponse, error) {
	response := GetAvailableLoansResponse{}
	err := c.executeRequest("GET", "/types/loans", nil, &response)
	if err != nil {
		return GetAvailableLoansResponse{}, err
	}

	return response, nil
}

// TODO: Get available structures
// TODO: Get info on available ships

// ////////////////////////////////////////////
// /// WARP JUMP
// ////////////////////////////////////////////

func (c *Client) WarpJump(shipId string) (WarpJumpResponse, error) {
	request := WarpJumpRequest{
		ShipId: shipId,
	}
	requestJson, err := json.Marshal(request)
	if err != nil {
		return WarpJumpResponse{}, err
	}

	response := WarpJumpResponse{}
	err = c.executeRequest("POST", "/my/warp-jumps", bytes.NewReader(requestJson), &response)
	if err != nil {
		return WarpJumpResponse{}, err
	}

	return response, nil
}
