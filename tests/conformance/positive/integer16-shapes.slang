// TypeCastIntVal truncates conversions; identity constructors preserve shape intermediates.
static const uint16_t Explicit=uint16_t(65538);
static const uint16_t Implicit=65538;
static const int16_t Signed=65538;
static const uint16_t Wide=uint16_t(65535)+uint16_t(3);
uniform float values[Implicit];
uint tag(float2 value){return 2;}
uint tag(float3 value){return 3;}
uint tag(int16_t2x3 value){return 23;}
RWStructuredBuffer<uint> output;
[shader("compute")][numthreads(1,1,1)]void main()
{
    output[0]=tag(vector<float,Explicit>(0));
    output[1]=tag(vector<float,Implicit>(0));
    output[2]=tag(vector<float,uint16_t(Wide)==2?3:2>(0));
    output[3]=tag(vector<float,uint16_t(int(Wide))==2?3:2>(0));
    output[4]=tag(matrix<int16_t,Signed,uint16_t(65539)>(0));
    const uint16_t Local=65538;
    output[5]=tag(vector<float,Local>(0));
}
