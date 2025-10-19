"""
User model.
"""

from dataclasses import dataclass
from typing import Optional
import time

@dataclass
class User:
    """User data class"""
    id: int
    username: str
    email: str
    created_at: float
    is_active: bool = True

    @classmethod
    def create(cls, username: str, email: str) -> 'User':
        """Create a new user"""
        return cls(
            id=int(time.time() * 1000),  # Simple ID generation
            username=username,
            email=email,
            created_at=time.time()
        )

    def deactivate(self):
        """Deactivate the user"""
        self.is_active = False

    def update_email(self, new_email: str):
        """Update user email"""
        self.email = new_email

    @property
    def display_name(self) -> str:
        """Get display name"""
        return f"{self.username} <{self.email}>"

class UserRepository:
    """Repository for user operations"""

    def __init__(self):
        self.users: dict[int, User] = {}

    def save(self, user: User):
        """Save a user"""
        self.users[user.id] = user

    def find_by_id(self, user_id: int) -> Optional[User]:
        """Find user by ID"""
        return self.users.get(user_id)

    def find_by_username(self, username: str) -> Optional[User]:
        """Find user by username"""
        for user in self.users.values():
            if user.username == username:
                return user
        return None

    def delete(self, user_id: int) -> bool:
        """Delete a user"""
        if user_id in self.users:
            del self.users[user_id]
            return True
        return False