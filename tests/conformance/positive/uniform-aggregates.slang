struct Params { float scale; float2 offset; };
typealias Values = Params;

uniform Values values[2];
uniform float2x2 transform;
uniform half2 weights;
uniform bool enabled;
RWStructuredBuffer<float> output;

[shader("compute")]
[numthreads(1, 1, 1)]
void main()
{
    output[0] = enabled ? values[0].scale + values[1].offset.x
        + transform[0][1] + float(weights.x) : 0;
}
