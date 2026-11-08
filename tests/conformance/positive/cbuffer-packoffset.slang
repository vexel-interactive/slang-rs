cbuffer Params : register(b2, space1)
{
    float scale : packoffset(c2.y);
    float tail;
    float2 values : packoffset(c0.z);
    float alias : packoffset(c2.y);
};
RWStructuredBuffer<float> output;
[shader("compute")]
[numthreads(1,1,1)]
void main() { output[0] = scale + tail + values.x + alias; }
