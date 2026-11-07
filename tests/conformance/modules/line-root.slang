#line 90 "virtual/root.slang"
import "line-library.slang";
RWStructuredBuffer<int> output;
[shader("compute")]
[numthreads(1, 1, 1)]
void main()
{
    output[0] = readLine() + __LINE__;
}
