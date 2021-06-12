package main

import (
	"fmt"
	"log"
	"os"
	"spacemonger"
)

func main() {
	for _, e := range os.Environ() {
		fmt.Println(e)
	}

	fmt.Println("Daemon Main")
	c, err := spacemonger.NewClient()
	if err != nil {
		log.Fatalln(err)
	}

	myIp, err := c.GetMyIpAddress()
	if err != nil {
		log.Fatalln(err)
	}

	fmt.Printf("MyIp: %+v\n", myIp)

	status, err := c.GetGameStatus()
	if err != nil {
		log.Fatalln(err)
	}

	fmt.Printf("Game Status: %+v\n", status)

	// claimedUsername, err := c.ClaimUsername("bloveless-dummy-username-test")
	// if err != nil {
	// 	log.Fatalln(err)
	// }

	claimedUsername := spacemonger.ClaimUsernameResponse{
		Token: "3d472a71-33f9-4752-a38c-761db39425c7",
		User: spacemonger.ClaimUsernameUser{
			Username: "bloveless-dummy-username-test",
			Credits: 0,
			Ships: []spacemonger.Ship{},
			Loans: []spacemonger.Loan{},
		},
	}

	fmt.Printf("New Username: %+v\n", claimedUsername)
}
