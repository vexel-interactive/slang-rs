import "predefined-library.slang";
RWStructuredBuffer<int> result;

[shader("compute")]
[numthreads(1, 1, 1)]
void main()
{
    result[0] = sourceLine() + __LINE__;
}
