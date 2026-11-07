#line 100 "virtual/root.slang"
#include "../include/line-header.slang"
static const int rootLine = HEADER_LINE;
RWStructuredBuffer<int> output;
[shader("compute")]
[numthreads(1, 1, 1)]
void main()
{
    output[0] = rootLine + headerLine;
}
