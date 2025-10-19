"""
User management functionality.
"""

from typing import List, Dict
import json

class UserManager:
    """Manages user accounts"""

    def __init__(self):
        self.users: Dict[str, str] = {}  # username -> email

    def add_user(self, username: str, email: str) -> bool:
        """Add a new user"""
        if username in self.users:
            return False
        self.users[username] = email
        return True

    def remove_user(self, username: str) -> bool:
        """Remove a user"""
        if username not in self.users:
            return False
        del self.users[username]
        return True

    def list_users(self) -> List[str]:
        """List all usernames"""
        return list(self.users.keys())

    def get_user_email(self, username: str) -> str:
        """Get user email"""
        return self.users.get(username, "")

    def save_to_file(self, filename: str):
        """Save users to JSON file"""
        with open(filename, 'w') as f:
            json.dump(self.users, f)

    def load_from_file(self, filename: str):
        """Load users from JSON file"""
        try:
            with open(filename, 'r') as f:
                self.users = json.load(f)
        except FileNotFoundError:
            pass