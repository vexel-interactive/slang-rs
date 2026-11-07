#define NUMBER 40
#define NAME "generated.slang"
#line NUMBER NAME
#if __LINE__ != 40
#error wrong remapped line
#endif
#line 8
static const int width = __LINE__;
RWStructuredBuffer<int> output;
[shader("compute")]
[numthreads(width, 1, 1)]
void main(uint index : SV_GroupIndex)
{
    output[index] = __LINE__;
}
