package tests

import (
	"fmt"
	"testing"
)

func stringNotBlank(t *testing.T, name string, value string) {
	if value == "" {
		t.Fatal(fmt.Sprintf("\"%s\" was blank and should not have been", name))
	}
}

func stringEquals(t *testing.T, name, value, expected string) {
	if value != expected {
		t.Fatal(fmt.Sprintf("expected \"%s\" to equal \"%s\" but it was \"%s\"", name, expected, value))
	}
}

