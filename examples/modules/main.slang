import math_utils;

RWStructuredBuffer<float> result;

[shader("compute")]
[numthreads(1, 1, 1)]
void main()
{
    Pair pair;
    pair.left = 2.0;
    pair.right = 3.0;
    result[0] = weightedSum(pair);
}
