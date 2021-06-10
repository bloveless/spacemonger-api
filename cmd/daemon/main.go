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

	fmt.Println(status)
}
