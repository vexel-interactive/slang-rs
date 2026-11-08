typedef vector<float,Count> Value;
static const uint Count = Next + 1u;
static const uint Next = (0xffffffffu + 1u) == 0u ? 2u : 1u;
ConstantBuffer<Value> input;
RWStructuredBuffer<float> output;
[shader("compute")]
[numthreads(1,1,1)]
void main()
{
    const int Count = 2;
    vector<float,Count> local = 1;
    output[0] = local.y + input.y;
}
