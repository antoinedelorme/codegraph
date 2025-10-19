"""
Utility functions.
"""

import hashlib
from typing import Any

def helper_function(input_str: str) -> str:
    """A helper function that processes strings"""
    return f"Processed: {input_str.upper()}"

def calculate_hash(data: str) -> str:
    """Calculate SHA256 hash of data"""
    return hashlib.sha256(data.encode()).hexdigest()

class DataProcessor:
    """Processes data"""

    def __init__(self, multiplier: int = 1):
        self.multiplier = multiplier

    def process(self, value: int) -> int:
        """Process an integer value"""
        return value * self.multiplier

    def process_list(self, values: list) -> list:
        """Process a list of values"""
        return [self.process(v) for v in values]

# Global variable
DEFAULT_PROCESSOR = DataProcessor(2)

def get_default_processor() -> DataProcessor:
    """Get the default processor"""
    return DEFAULT_PROCESSOR