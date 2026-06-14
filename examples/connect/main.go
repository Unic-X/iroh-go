package main

import (
	"log"

	"go-iroh/iroh"
)

func main() {
	ep, err := iroh.NewEndpoint()
	if err != nil {
		log.Fatal(err)
	}

	defer ep.Close()

	conn, err := ep.Connect(
		"NODE_ID",
	)
	if err != nil {
		log.Fatal(err)
	}

	log.Println(conn)
}
