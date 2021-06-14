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

	// claimedUsername, err := c.ClaimUsername("blove-go-test")
	// if err != nil {
	// 	log.Fatalln(err)
	// }

	// claimedUsername := spacemonger.ClaimUsernameResponse{
	// 	Token: "3d472a71-33f9-4752-a38c-761db39425c7",
	// 	User: spacemonger.ClaimUsernameResponseUser{
	// 		Username: "bloveless-dummy-username-test",
	// 		Credits:  0,
	// 		Ships:    []spacemonger.Ship{},
	// 		Loans:    []spacemonger.Loan{},
	// 	},
	// }
	//
	// fmt.Printf("New Username: %+v\n", claimedUsername)

	// username := "blove-go-test"
	token := "c53e4835-d8cc-4579-b7d5-99b1df31bf8e"

	c.SetToken(token)

	myInfo, err := c.GetMyInfo()
	if err != nil {
		log.Fatalf("GetMyInfo error: %+v", err)
	}

	fmt.Printf("GetMyInfo data: %+v", myInfo)

	if myInfo.User.Credits == 0 {
		createLoanResponse, err := c.CreateLoan(spacemonger.StartUpLoan)
		if err != nil {
			panic(err)
		}

		fmt.Printf("New Loan: %+v\n", createLoanResponse)
	}

	myLoans, err := c.GetMyLoans()
	if err != nil {
		panic(err)
	}

	fmt.Printf("My Loans: %+v\n", myLoans)

}
