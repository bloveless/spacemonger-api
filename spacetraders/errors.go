package spacetraders

import (
	"errors"
	"fmt"
)

var TooManyRetriesError = errors.New("too many retries")
var InvalidRequestError = errors.New("the given request was invalid")
var UnauthorizedError = errors.New("unauthorized")
var UnableToDecodeResponseError = errors.New("unable to decode response")
var MaintenanceModeError = errors.New("server is in maintenance mode")

type SpaceTraderErrorMessage struct {
	Message string `json:"message"`
	Code    int    `json:"code"`
}

type SpaceTraderError struct {
	ApiError SpaceTraderErrorMessage `json:"error"`
}

func (e *SpaceTraderError) Error() string {
	return fmt.Sprintf("A spacetraders error occurred. Message: %s, Code: %d", e.ApiError.Message, e.ApiError.Code)
}
