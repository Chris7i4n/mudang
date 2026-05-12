def preamble_probe():
    return "ok"


from functools import lru_cache


@lru_cache(maxsize=128,

def expensive_op(x):
    return x * x


@lru_cache(maxsize=64)
def cheaper_op(y):
    return y * 2
