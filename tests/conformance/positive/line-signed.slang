#line 4294967295 "generated.slang"
#if __LINE__ != -1
#error wrong signed line
#endif
#line 2147483648
#if __LINE__ != -2147483648
#error wrong minimum line
#endif
#line 0
static const int zero = __LINE__;
RWStructuredBuffer<int> output;
[shader("compute")]
[numthreads(1, 1, 1)]
void main()
{
    output[0] = zero;
}
