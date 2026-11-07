StructuredBuffer<int> data;
[shader("compute")]
[numthreads(1, 1, 1)]
void main()
{
    InterlockedAdd(data[0], 1);
}
