# Message cookbook

This book contains recipes for writing messages for vnix services.

## Services

### Math

#### math.int
This service is designed to compute arbitrary precision integers and integer tensors (vector, matrix, multidimensional matrix).

1. Unary operations:

Request1:
```
{
    val: {
        <neg | abs | inc | dec | sqr | sqrt | fac : str>:<value: int | map>
    }
}
```

Response1:
```
{val:<result: int>}
```

Examples:
```
{neg:5} # -5

{abs:-5} # |-5| = 5

{inc:5} # 5++ = 6

{dec:5} # 5-- = 4

{sqr:5} # 5 ^ 2 = 25

{sqrt:5} # sqrt(5) = 2

{fac:5} # 5! = 120
```
```
# -(5! ^ 2) = -14400
{
    val: {
        neg:{
            sqr:{fac:5}
        }
    }
}
```

Request2:
```
{
    val: {
        fact:<value: int | map>
    }
}
```

Response2:
```
{val:<primes: list<int>>}
```

Example:
```
# 126 = [2 3 3 7]
{
    val:{fact:126}
}
```

2. Reduce operations:

Request:
```
{
    val:{
        <sum | sub | mul | div | mod | pow : str>:<values: pair<int | map> | list<int | map>>
    }
}
```

Response:
```
{val:<result: int>}
```

Examples:
```
{sum:(2 3)} # 2 + 3 = 5

{sub:[5 4 1]} # 5 - 4 - 1 = 0

{mul:(2 3)} # 2 * 3 = 6

{div:[10 2 2]} # (10 / 2) / 2 = 5 / 2 = 2

{mod:(5 2)} # 5 mod 2 = 1

{pow:[7 3 2]} # (7 ^ 3) ^ 2 = 117649
```

```
# (5 ^ 7) ^ 2 mod 3 = 1
{
    val:{
        mod:({pow:[5 7 2]} 3)
    }
}
```
