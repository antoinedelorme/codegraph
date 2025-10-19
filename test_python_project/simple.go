package main

import "fmt"

// User represents a user
type User struct {
	ID   int
	Name string
}

// NewUser creates a new user
func NewUser(id int, name string) *User {
	return &User{
		ID:   id,
		Name: name,
	}
}

// Display prints user information
func (u *User) Display() string {
	return fmt.Sprintf("User %d: %s", u.ID, u.Name)
}

// Helper function
func helper(msg string) string {
	return "Helper: " + msg
}

const Version = "1.0.0"

var GlobalCounter int

func main() {
	user := NewUser(1, "Alice")
	fmt.Println(user.Display())

	result := helper("test")
	fmt.Println(result)

	GlobalCounter++
}