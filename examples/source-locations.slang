#define HERE __LINE__
#define ID(value) value

#if !defined(__LINE__) || !defined(__FILE__) || !__SLANG__
#error Required Slang source macros are missing
#endif

static const uint declarationLine = HERE;
RWStructuredBuffer<uint> result;

[shader("compute")]
[numthreads(1, 1, 1)]
void main()
{
    result[0] = declarationLine;
    result[1] = ID(
        __LINE__
    );
}
