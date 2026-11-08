import uniform_library;
struct Params { float scale; };

uniform float rootValue;
static float privateValue = 1;
groupshared float sharedValue;
ConstantBuffer<Params> fixed : register(b0);
ConstantBuffer<Params> automatic;
RWStructuredBuffer<float> output;

[shader("compute")]
[numthreads(1, 1, 1)]
void main()
{
    sharedValue = privateValue;
    output[0] = rootValue + importedValue + sharedValue + fixed.scale + automatic.scale;
}
