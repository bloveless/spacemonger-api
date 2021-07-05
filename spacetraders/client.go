package spacetraders

import (
	"bytes"
	"context"
	"encoding/json"
	"fmt"
	"io/ioutil"
	"log"
	"net/http"
	"net/url"
	"os"
	"strconv"
	"sync"
	"time"
)

// It doesn't matter how many of these clients we have... the rate limit will apply to all of the so we need a global
// mutex that we can lock all the clients at the same time
var httpMutex sync.Mutex

type Client struct {
	httpClient http.Client
	baseURL    string
}

func NewClient() (Client, error) {
	transport := &http.Transport{}

	envProxy := os.Getenv("HTTP_PROXY")
	if envProxy != "" {
		proxy, err := url.Parse(envProxy)
		if err != nil {
			return Client{}, fmt.Errorf("unable to parse HTTP_PROXY as a url: %w", err)
		}
		transport.Proxy = http.ProxyURL(proxy)
	}

	return Client{
		httpClient: http.Client{
			Transport: transport,
			Timeout:   time.Second * 10,
		},
		baseURL: "https://api.spacetraders.io",
	}, nil
}

func executeRequest(ctx context.Context, client Client, method string, url string, token string, body []byte, decodeResponse interface{}) error {
	// TODO: To mutex or not to mutex
	httpMutex.Lock()
	defer httpMutex.Unlock()

	attemptCount := 0
	for {
		attemptCount += 1

		request, err := http.NewRequestWithContext(ctx, method, url, bytes.NewReader(body))
		if err != nil {
			return fmt.Errorf("unable to create a new request with context: %w", err)
		}

		request.Header.Add("Content-Type", "application/json")
		if token != "" {
			request.Header.Add("Authorization", fmt.Sprintf("Bearer %s", token))
		}

		if attemptCount > 3 {
			return TooManyRetriesError
		}

		response, err := client.httpClient.Do(request)
		if err != nil {
			return fmt.Errorf("unable to execute http request: %w", err)
		}

		responseBody, err := ioutil.ReadAll(response.Body)
		response.Body.Close()
		if err != nil {
			return fmt.Errorf("unable to read response body: %w", err)
		}

		if response.StatusCode >= 200 && response.StatusCode < 300 {
			if err := json.Unmarshal(responseBody, decodeResponse); err != nil {
				log.Printf("Json Unmarshal Err: %+v\n", err)
				return UnableToDecodeResponseError
			}

			return nil
		}

		if response.StatusCode == http.StatusServiceUnavailable {
			return MaintenanceModeError
		}

		if response.StatusCode == http.StatusUnauthorized {
			return UnauthorizedError
		}

		if response.StatusCode == http.StatusTooManyRequests {
			retryAfter, err := strconv.ParseFloat(response.Header.Get("retry-after"), 64)
			if err != nil {
				return fmt.Errorf("unable to parse retry-after header as float64: %w", err)
			}

			waitTime := time.Duration(retryAfter*1000) * time.Millisecond
			log.Printf("Rate limited... waiting for %v seconds before trying again. Request: \"%s %s\"\n", waitTime, method, url)

			time.Sleep(waitTime)
			continue
		}

		if response.StatusCode == 500 {
			// If there was an internal server error then try the request again in 2 seconds
			log.Printf("Caught internal server error retrying in 2 seconds. %s", responseBody)
			time.Sleep(2 * time.Second)

			continue
		}

		e := &SpaceTraderError{}
		err = json.Unmarshal(responseBody, &e)
		if err != nil {
			log.Printf("ERROR unmarshalling response body. Response body: %s", string(responseBody))
			return fmt.Errorf("unable to unmarshal response body: %w", err)
		}
		return e
	}
}

func (c *Client) SetBaseURL(baseURL string) {
	c.baseURL = baseURL
}

// GetMyIpAddress will get the clients current external ip address
func (c Client) GetMyIpAddress(ctx context.Context) (GetMyIpAddressResponse, error) {
	response := GetMyIpAddressResponse{}
	err := executeRequest(ctx, c, "GET", "https://api.ipify.org?format=json", "", nil, &response)
	if err != nil {
		return GetMyIpAddressResponse{}, fmt.Errorf("unable to get ip address: %w", err)
	}

	return response, nil
}

// ClaimUsername will claim a username and return a token
func (c Client) ClaimUsername(ctx context.Context, username string) (ClaimUsernameResponse, error) {
	response := ClaimUsernameResponse{}
	err := executeRequest(ctx, c, "POST", c.baseURL+fmt.Sprintf("/users/%s/token", username), "", nil, &response)
	if err != nil {
		return ClaimUsernameResponse{}, fmt.Errorf("unable to claim username \"%s\": %w", username, err)
	}

	return response, nil
}

// GetGameStatus will return the current status of https://api.spacetraders.io
func (c Client) GetGameStatus(ctx context.Context) (GameStatusResponse, error) {
	response := GameStatusResponse{}
	err := executeRequest(ctx, c, "GET", c.baseURL+"/game/status", "", nil, &response)
	if err != nil {
		return GameStatusResponse{}, fmt.Errorf("unable to get game status: %w", err)
	}

	return response, nil
}

type AuthorizedClient struct {
	client Client
	token  string
}

func NewAuthorizedClient(client Client, token string) (AuthorizedClient, error) {
	client, err := NewClient()
	if err != nil {
		return AuthorizedClient{}, fmt.Errorf("unable to create new authorized client: %w", err)
	}

	return AuthorizedClient{
		client,
		token,
	}, nil
}

// ////////////////////////////////////////////
// /// ACCOUNT
// ////////////////////////////////////////////

// GetMyInfo returns the current users info
func (ac AuthorizedClient) GetMyInfo(ctx context.Context) (GetMyInfoResponse, error) {
	r := GetMyInfoResponse{}
	err := executeRequest(ctx, ac.client, "GET", ac.client.baseURL+"/my/account", ac.token, nil, &r)
	if err != nil {
		return GetMyInfoResponse{}, fmt.Errorf("unable to get my info: %w", err)
	}

	return r, nil
}

// ////////////////////////////////////////////
// /// FLIGHT PLANS
// ////////////////////////////////////////////

func (ac AuthorizedClient) GetFlightPlan(ctx context.Context, flightPlanId string) (GetFlightPlanResponse, error) {
	response := GetFlightPlanResponse{}
	err := executeRequest(ctx, ac.client, "GET", ac.client.baseURL+fmt.Sprintf("/my/flight-plans/%s", flightPlanId), ac.token, nil, &response)

	if err != nil {
		return GetFlightPlanResponse{}, fmt.Errorf("unable to get flight plan \"%s\": %w", flightPlanId, err)
	}

	return response, nil
}

func (ac AuthorizedClient) CreateFlightPlan(ctx context.Context, shipId, destination string) (CreateFlightPlanResponse, error) {
	request := CreateFlightPlanRequest{
		ShipId:      shipId,
		Destination: destination,
	}
	requestJson, err := json.Marshal(request)
	if err != nil {
		log.Printf("ERROR creating flight plan request: %+v", request)
		return CreateFlightPlanResponse{}, fmt.Errorf("unable to marshal flight plan request: %w", err)
	}

	response := CreateFlightPlanResponse{}
	err = executeRequest(ctx, ac.client, "POST", ac.client.baseURL+"/my/flight-plans", ac.token, requestJson, &response)
	if err != nil {
		return CreateFlightPlanResponse{}, fmt.Errorf("unable to create flight plan for ship id \"%s\" to \"%s\": %w", shipId, destination, err)
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

func (ac AuthorizedClient) GetMyLoans(ctx context.Context) (GetMyLoansResponse, error) {
	response := GetMyLoansResponse{}
	err := executeRequest(ctx, ac.client, "GET", ac.client.baseURL+"/my/loans", ac.token, nil, &response)
	if err != nil {
		return GetMyLoansResponse{}, fmt.Errorf("unable to get my loans: %w", err)
	}

	return response, nil
}

func (ac AuthorizedClient) PayOffLoan(ctx context.Context, loanId string) (PayOffLoanResponse, error) {
	response := PayOffLoanResponse{}
	// TODO: If this request doesn't work then it likely needs a body of any valid json payload
	err := executeRequest(ctx, ac.client, "PUT", ac.client.baseURL+fmt.Sprintf("/my/loans/%s", loanId), ac.token, nil, &response)
	if err != nil {
		return PayOffLoanResponse{}, fmt.Errorf("unable to pay off loan \"%s\": %w", loanId, err)
	}

	return response, nil
}

func (ac AuthorizedClient) CreateLoan(ctx context.Context, loanType string) (CreateLoanResponse, error) {
	request := CreateLoanRequest{
		Type: loanType,
	}
	requestJson, err := json.Marshal(request)
	if err != nil {
		log.Printf("ERROR marshalling create loan request: %+v", request)
		return CreateLoanResponse{}, fmt.Errorf("unable to marshal create loan request: %w", err)
	}

	response := CreateLoanResponse{}
	err = executeRequest(ctx, ac.client, "POST", ac.client.baseURL+"/my/loans", ac.token, requestJson, &response)
	if err != nil {
		return CreateLoanResponse{}, fmt.Errorf("unable to create loan \"%s\": %w", loanType, err)
	}

	return response, nil
}

// ////////////////////////////////////////////
// /// LOCATIONS
// ////////////////////////////////////////////

func (ac AuthorizedClient) GetLocation(ctx context.Context, location string) (GetLocationResponse, error) {
	response := GetLocationResponse{}
	err := executeRequest(ctx, ac.client, "GET", ac.client.baseURL+fmt.Sprintf("/locations/%s", location), ac.token, nil, &response)
	if err != nil {
		return GetLocationResponse{}, fmt.Errorf("unable to get location \"%s\": %w", location, err)
	}

	return response, nil
}

func (ac AuthorizedClient) GetLocationMarketplace(ctx context.Context, location string) (GetLocationMarketplaceResponse, error) {
	response := GetLocationMarketplaceResponse{}
	err := executeRequest(ctx, ac.client, "GET", ac.client.baseURL+fmt.Sprintf("/locations/%s/marketplace", location), ac.token, nil, &response)
	if err != nil {
		return GetLocationMarketplaceResponse{}, fmt.Errorf("unable to get location marketplace for \"%s\": %w", location, err)
	}

	return response, nil
}

// TODO: Get Ships at a location

// ////////////////////////////////////////////
// /// PURCHASE ORDERS
// ////////////////////////////////////////////

func (ac AuthorizedClient) CreatePurchaseOrder(ctx context.Context, shipId, good string, quantity int) (CreatePurchaseOrderResponse, error) {
	request := CreatePurchaseOrderRequest{
		ShipId:   shipId,
		Good:     good,
		Quantity: quantity,
	}
	requestJson, err := json.Marshal(request)
	if err != nil {
		log.Printf("ERROR marshalling create purchase order request: %+v", request)
		return CreatePurchaseOrderResponse{}, fmt.Errorf("unable to marshal create purchase order request: %w", err)
	}

	response := CreatePurchaseOrderResponse{}
	err = executeRequest(ctx, ac.client, "POST", ac.client.baseURL+"/my/purchase-orders", ac.token, requestJson, &response)
	if err != nil {
		return CreatePurchaseOrderResponse{}, fmt.Errorf("unable to create purchase order: %w", err)
	}

	return response, nil
}

// ////////////////////////////////////////////
// /// SELL ORDERS
// ////////////////////////////////////////////

func (ac AuthorizedClient) CreateSellOrder(ctx context.Context, shipId, good string, quantity int) (CreateSellOrderResponse, error) {
	request := CreateSellOrderRequest{
		ShipId:   shipId,
		Good:     good,
		Quantity: quantity,
	}
	requestJson, err := json.Marshal(request)
	if err != nil {
		log.Printf("ERROR marshalling create sell order request: %+v", request)
		return CreateSellOrderResponse{}, fmt.Errorf("unable to marshal create sell order request: %w", err)
	}

	response := CreateSellOrderResponse{}
	err = executeRequest(ctx, ac.client, "POST", ac.client.baseURL+"/my/sell-orders", ac.token, requestJson, &response)
	if err != nil {
		return CreateSellOrderResponse{}, fmt.Errorf("unable to create sell order: %w", err)
	}

	return response, nil
}

// ////////////////////////////////////////////
// /// SHIPS
// ////////////////////////////////////////////

func (ac AuthorizedClient) PurchaseShip(ctx context.Context, location, shipType string) (PurchaseShipResponse, error) {
	request := PurchaseShipRequest{
		Location: location,
		Type:     shipType,
	}
	requestJson, err := json.Marshal(request)
	if err != nil {
		log.Printf("ERROR marshalling purchase ship request: %+v", request)
		return PurchaseShipResponse{}, fmt.Errorf("unable to marshal purchase ship request: %w", err)
	}

	response := PurchaseShipResponse{}
	err = executeRequest(ctx, ac.client, "POST", ac.client.baseURL+"/my/ships", ac.token, requestJson, &response)
	if err != nil {
		return PurchaseShipResponse{}, fmt.Errorf("unable to purchase ship: %w", err)
	}

	return response, nil
}

func (ac AuthorizedClient) GetMyShip(ctx context.Context, shipId string) (GetMyShipRequest, error) {
	response := GetMyShipRequest{}
	err := executeRequest(ctx, ac.client, "GET", ac.client.baseURL+fmt.Sprintf("/my/ships/%s", shipId), ac.token, nil, &response)
	if err != nil {
		return GetMyShipRequest{}, fmt.Errorf("unable to get my ship \"%s\": %w", shipId, err)
	}

	return response, nil
}

func (ac AuthorizedClient) GetMyShips(ctx context.Context) (GetMyShipsResponse, error) {
	response := GetMyShipsResponse{}
	err := executeRequest(ctx, ac.client, "GET", ac.client.baseURL+"/my/ships", ac.token, nil, &response)
	if err != nil {
		return GetMyShipsResponse{}, fmt.Errorf("unable to get my ships: %w", err)
	}

	return response, nil
}

func (ac AuthorizedClient) JettisonCargo(ctx context.Context, shipId string, good string, quantity int) (JettisonCargoResponse, error) {
	request := JettisonCargoRequest{
		Good:     good,
		Quantity: quantity,
	}
	requestJson, err := json.Marshal(request)
	if err != nil {
		log.Printf("ERROR marshalling jettison cargo request: %+v", request)
		return JettisonCargoResponse{}, fmt.Errorf("unable to marshal jettison cargo request: %w", err)
	}

	response := JettisonCargoResponse{}
	err = executeRequest(ctx, ac.client, "POST", ac.client.baseURL+fmt.Sprintf("/my/ships/%s/jettison", shipId), ac.token, requestJson, &response)
	if err != nil {
		return JettisonCargoResponse{}, fmt.Errorf("unable to jettison cargo: %w", err)
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

func (ac AuthorizedClient) GetShipsForSale(ctx context.Context) (GetShipsForSaleResponse, error) {
	response := GetShipsForSaleResponse{}
	err := executeRequest(ctx, ac.client, "GET", ac.client.baseURL+"/game/ships", ac.token, nil, &response)
	if err != nil {
		return GetShipsForSaleResponse{}, fmt.Errorf("unable to get ships for sale: %w", err)
	}

	return response, nil
}

// TODO: Get all active flight plans in the system.
// TODO: Get info on a system's docked ships

func (ac AuthorizedClient) GetSystemLocations(ctx context.Context, system string) (GetSystemLocationsResponse, error) {
	response := GetSystemLocationsResponse{}
	err := executeRequest(ctx, ac.client, "GET", ac.client.baseURL+fmt.Sprintf("/systems/%s/locations", system), ac.token, nil, &response)
	if err != nil {
		return GetSystemLocationsResponse{}, fmt.Errorf("unable to get system locations: %w", err)
	}

	return response, nil
}

func (ac AuthorizedClient) GetSystem(ctx context.Context, system string) (GetSystemResponse, error) {
	response := GetSystemResponse{}
	err := executeRequest(ctx, ac.client, "GET", ac.client.baseURL+fmt.Sprintf("/systems/%s", system), ac.token, nil, &response)
	if err != nil {
		return GetSystemResponse{}, fmt.Errorf("unable to get system \"%s\": %w", system, err)
	}

	return response, nil
}

// ////////////////////////////////////////////
// /// TYPES
// ////////////////////////////////////////////

// TODO: Get available goods

func (ac AuthorizedClient) GetAvailableLoans(ctx context.Context) (GetAvailableLoansResponse, error) {
	response := GetAvailableLoansResponse{}
	err := executeRequest(ctx, ac.client, "GET", ac.client.baseURL+"/types/loans", ac.token, nil, &response)
	if err != nil {
		return GetAvailableLoansResponse{}, fmt.Errorf("unable to get available loans: %w", err)
	}

	return response, nil
}

// TODO: Get available structures
// TODO: Get info on available ships

// ////////////////////////////////////////////
// /// WARP JUMP
// ////////////////////////////////////////////

func (ac AuthorizedClient) WarpJump(ctx context.Context, shipId string) (WarpJumpResponse, error) {
	request := WarpJumpRequest{
		ShipId: shipId,
	}
	requestJson, err := json.Marshal(request)
	if err != nil {
		log.Printf("ERROR marshalling warp jump request: %+v", request)
		return WarpJumpResponse{}, fmt.Errorf("unable to marshal warp jump request: %w", err)
	}

	response := WarpJumpResponse{}
	err = executeRequest(ctx, ac.client, "POST", ac.client.baseURL+"/my/warp-jumps", ac.token, requestJson, &response)
	if err != nil {
		return WarpJumpResponse{}, fmt.Errorf("unable to warp jump: %w", err)
	}

	return response, nil
}
