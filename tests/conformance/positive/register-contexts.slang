struct Parameters { float x : register(c2.y); float y : register(b4); };
uniform Parameters data : register(b2);
float scale : register(t3);
static float storage : register(c4) = 1;
groupshared float shared : register(b7);
cbuffer Params { float value : register(c5.y); };
float evaluate(float input : register(c2)) : register(t2)
{
    return input + data.x + scale + value + storage;
}
RWStructuredBuffer<float> output;
[shader("compute")]
[numthreads(1,1,1)]
void main() { shared = 2; output[0] = evaluate(shared); }
