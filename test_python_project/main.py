#!/usr/bin/env python3
"""
Main entry point for the test application.
"""

import sys
from user_manager import UserManager
from utils import helper_function

def main():
    """Main function"""
    print("Starting test application...")

    # Create user manager
    manager = UserManager()

    # Add some users
    manager.add_user("alice", "alice@example.com")
    manager.add_user("bob", "bob@example.com")

    # List users
    users = manager.list_users()
    print(f"Users: {users}")

    # Use helper function
    result = helper_function("test")
    print(f"Helper result: {result}")

    return 0

if __name__ == "__main__":
    sys.exit(main())