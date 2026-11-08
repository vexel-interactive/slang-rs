cbuffer Params
{
    double wide : packoffset(c0.y);
    float tail;
    float values[2] : packoffset(c2);
    float after;
    static float storage : packoffset(c7) = 1;
};
RWStructuredBuffer<float> output;
[shader("compute")]
[numthreads(1,1,1)]
void main() { output[0] = float(wide) + tail + values[1] + after + storage; }
