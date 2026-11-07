#undef __LINE__
RWStructuredBuffer<int> result;

[shader("compute")]
[numthreads(1, 1, 1)]
void main()
{
    result[0] = __LINE__;
}
