package spacemonger

import (
	"errors"
	"fmt"
)

var TooManyRetries = errors.New("too many retries")

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