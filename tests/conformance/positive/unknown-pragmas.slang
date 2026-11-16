#define NAME once
#define BAD(a, b) a + b

// Names and payloads of unknown pragmas are not macro-expanded.
#pragma NAME
#pragma custom BAD(1)

#if 0
#pragma
#pragma 123
#pragma warning(push)
#endif

#define RESULT 7
RWStructuredBuffer<uint> output;

[shader("compute")]
[numthreads(1, 1, 1)]
void main()
{
    output[0] = RESULT;
}
