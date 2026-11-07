RWStructuredBuffer<int> result;

[shader("compute")]
[numthreads(1, 1, 1)]
void main()
{
    for (int i = 0, j = 2; i < 3; ++i, ++j)
    {
        result[0] = j;
    }
    result[0] = j;
}
