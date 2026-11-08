struct Params { float scale; };

uniform float value;
ConstantBuffer<Params> otherSpace : register(b0, space3);
ConstantBuffer<Params> automatic;
RWStructuredBuffer<float> output;

[shader("compute")]
[numthreads(1, 1, 1)]
void main()
{
    output[0] = value + otherSpace.scale + automatic.scale;
}
