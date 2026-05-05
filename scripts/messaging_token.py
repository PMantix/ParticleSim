"""messaging_token.py — emit a fresh random bearer token for the DOE
messaging server. Pipe to .doe_token (gitignored) and share with the
peer over the existing git messaging channel."""
import secrets
import sys

print(secrets.token_urlsafe(32))
