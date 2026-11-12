package main

import (
	"fmt"
	"io"
	"os"
)

func main() {
	root, err := os.Getwd()
	if err == nil {
		err = run(root, os.Args[1:])
	}
	if err != nil {
		fmt.Fprintln(os.Stderr, "error:", err)
		os.Exit(1)
	}
}

func run(root string, args []string) error {
	return runWithOutput(root, args, os.Stdout)
}

func runWithOutput(root string, args []string, output io.Writer) error {
	command := "build"
	if len(args) > 0 {
		command = args[0]
	}
	if command == "help" || command == "--help" || command == "-h" {
		topic := ""
		if len(args) > 1 {
			topic = args[1]
		}
		return printHelp(output, topic)
	}
	if len(args) > 1 && (command == "build" || command == "serve") {
		for _, arg := range args[1:] {
			if arg == "--help" || arg == "-h" {
				return printHelp(output, command)
			}
		}
	}
	switch command {
	case "build":
		return buildSite(root, includeDraftPosts(args))
	case "serve":
		return serveSite(root, args)
	case "new":
		return newCommand(root, args[1:], output)
	default:
		return fmt.Errorf("unknown command: %s. Run `go run . --help` for usage", command)
	}
}

func includeDraftPosts(args []string) bool {
	for _, arg := range args {
		if arg == "--draft" {
			return true
		}
	}
	return false
}

func previewAddress(args []string) string {
	port := "1313"
	for i := 0; i+1 < len(args); i++ {
		if args[i] == "--port" {
			port = args[i+1]
			break
		}
	}
	return "127.0.0.1:" + port
}
