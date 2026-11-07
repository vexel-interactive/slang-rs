RWStructuredBuffer<float> result;
[shader("compute")]
[numthreads(1, 1, 1)]
void main()
{
    vector<float2, 3> value;
    value[0] = float2(1.0);
    result[0] = value[0].x;
}
