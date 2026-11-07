#define HERE __LINE__
#define ID(x) x
#line 50 "generated.slang"
static const int first = HERE;
static const int second = ID(
    __LINE__
);
#line 10 \
"continued.slang"
static const int continued = __LINE__;
RWStructuredBuffer<int> output;
[shader("compute")]
[numthreads(1, 1, 1)]
void main()
{
    output[0] = first + second + continued;
}
