static var first = second;
static var second = first;
RWStructuredBuffer<int> output;
[shader("compute")]
[numthreads(1, 1, 1)]
void main() { output[0] = first; }
