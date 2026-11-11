import functools


def retry(times):
    def decorator(function):
        @functools.wraps(function)
        def wrapper(*args, **kwargs):
            for attempt in range(times):
                try:
                    return function(*args, **kwargs)
                except ValueError:
                    if attempt == times - 1:
                        raise
            return None
        return wrapper
    return decorator


class Account:
    limit = 100

    def __init__(self, owner, balance=0):
        self.owner = owner
        self.balance = balance

    @property
    def rich(self):
        return self.balance > Account.limit

    async def pay(self, amount, *, note=None):
        with open("log") as handle:
            handle.write(note or "")
        key = lambda item: item if item else 0
        return sorted([amount], key=key)
