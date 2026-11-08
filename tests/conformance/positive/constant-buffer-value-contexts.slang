ConstantBuffer<float> scalar;
ConstantBuffer<float2> vector;
ConstantBuffer<int> index;
ConstantBuffer<bool> condition;
typedef float3 Values[2];
ConstantBuffer<Values> input;
RWStructuredBuffer<float> output;
[shader("compute")]
[numthreads(1,1,1)]
void main()
{
    float3 flat = { scalar, vector };
    float3 constructed = float3(vector, scalar);
    Values copy = Values(input);
    float value = sin(scalar) + dot(vector, vector) + input[index].z;
    int selected = index;
    switch(selected) { case 0: value++; break; default: break; }
    if(condition) value += flat.x;
    output[0] = condition ? value + constructed.z + copy[0].y : scalar;
}
