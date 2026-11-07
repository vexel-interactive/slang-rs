groupshared int value;
RWStructuredBuffer<int> result;
[shader("compute")]
[numthreads(1, 1, 1)]
void main()
{
    int value = 0;
    InterlockedAdd(value, 1);
    result[0] = value;
}
