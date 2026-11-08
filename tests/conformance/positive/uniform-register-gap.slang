struct Params { float scale; };

uniform float value;
ConstantBuffer<Params> first : register(b0);
ConstantBuffer<Params> third : register(b2);
ConstantBuffer<Params> automatic;
RWStructuredBuffer<float> output;

[shader("compute")]
[numthreads(1, 1, 1)]
void main()
{
    output[0] = value + first.scale + third.scale + automatic.scale;
}
