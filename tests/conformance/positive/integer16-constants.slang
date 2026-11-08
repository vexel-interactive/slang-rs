// Typed truncation follows slang-ir.cpp; arithmetic preserves narrow component types.
static const int16_t Minimum=int16_t(32768);
static const uint16_t Maximum=uint16_t(65535);
static const uint16_t Wrapped=Maximum+uint16_t(1);
static const uint16_t Count=uint16_t(2);
static const int16_t SignedShift=Minimum>>2;
static const uint16_t UnsignedShift=Maximum>>13;
int16_t defaultValue(int16_t value=int16_t(65535)){return value;}
RWStructuredBuffer<uint> output;
[shader("compute")][numthreads(1,1,1)]void main()
{
    int16_t values[Count]={int16_t(-9),int16_t(-7)};
    int16_t value=values[0];uint16_t divisor=uint16_t(2);int16_t assigned=(value/=divisor);
    uint16_t unsignedValue=uint16_t(65535);uint16_t unsignedAssigned=(unsignedValue/=int16_t(-2));
    output[0]=uint(int16_t(65535));
    output[1]=uint(uint16_t(65536));
    output[2]=uint(int16_t(32767)+int16_t(1));
    output[3]=uint(uint16_t(65535)+uint16_t(1));
    output[4]=uint(int16_t(-9)/uint16_t(2));
    output[5]=uint(uint16_t(65535)/int16_t(-2));
    output[6]=uint(int16_t(-1)>>15);
    output[7]=uint(uint16_t(65535)>>15);
    output[8]=uint(int16_t(1)<<16);
    output[9]=uint(uint16_t(65535)>>16);
    output[10]=uint(countbits(int16_t(-1)));
    output[11]=uint(countbits(uint16_t(65535)));
    output[12]=uint(reversebits(int16_t(1)));
    output[13]=uint(reversebits(uint16_t(1)));
    output[14]=uint(reversebits(int16_t(-1)));
    output[15]=uint(firstbithigh(int16_t(-2)));
    output[16]=uint(firstbithigh(uint16_t(65535)));
    output[17]=uint(firstbitlow(int16_t(-32768)));
    output[18]=uint(asuint(int16_t(-1)));
    output[19]=uint(asuint(uint16_t(65535)));
    output[20]=uint(Minimum);
    output[21]=uint(Maximum);
    output[22]=uint(Wrapped);
    output[23]=uint(Count);
    output[24]=uint(SignedShift);
    output[25]=uint(UnsignedShift);
    output[26]=uint(assigned);
    output[27]=uint(value);
    output[28]=uint(unsignedAssigned);
    output[29]=uint(unsignedValue);
    output[30]=uint(values[1]);
    output[31]=uint(defaultValue());
    output[32]=uint(int16_t(0xffffffffffffffffULL));
    output[33]=uint(uint16_t(-1LL));
    output[34]=uint(int16_t(int16_t(200)*int16_t(200)));
    output[35]=uint(uint16_t(uint16_t(60000)+uint16_t(6000)));
    output[36]=uint(int16_t(int16_t(-32768)/int16_t(2)));
    output[37]=uint(uint16_t(uint16_t(65535)%uint16_t(7)));
    output[38]=uint(int16_t(~int16_t(0)));
    output[39]=uint(uint16_t(~uint16_t(0)));
}
