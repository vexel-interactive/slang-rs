// Fixed-width scalar/vector/matrix aliases from core.meta.slang.
uint choose(int16_t value){return 1u;}
uint choose(int16_t1 value){return 2u;}
uint choose(int16_t2 value){return 3u;}
uint choose(int16_t3 value){return 4u;}
uint choose(int16_t4 value){return 5u;}
uint choose(int16_t1x1 value){return 6u;}
uint choose(int16_t1x2 value){return 7u;}
uint choose(int16_t1x3 value){return 8u;}
uint choose(int16_t1x4 value){return 9u;}
uint choose(int16_t2x1 value){return 10u;}
uint choose(int16_t2x2 value){return 11u;}
uint choose(int16_t2x3 value){return 12u;}
uint choose(int16_t2x4 value){return 13u;}
uint choose(int16_t3x1 value){return 14u;}
uint choose(int16_t3x2 value){return 15u;}
uint choose(int16_t3x3 value){return 16u;}
uint choose(int16_t3x4 value){return 17u;}
uint choose(int16_t4x1 value){return 18u;}
uint choose(int16_t4x2 value){return 19u;}
uint choose(int16_t4x3 value){return 20u;}
uint choose(int16_t4x4 value){return 21u;}
uint choose(uint16_t value){return 22u;}
uint choose(uint16_t1 value){return 23u;}
uint choose(uint16_t2 value){return 24u;}
uint choose(uint16_t3 value){return 25u;}
uint choose(uint16_t4 value){return 26u;}
uint choose(uint16_t1x1 value){return 27u;}
uint choose(uint16_t1x2 value){return 28u;}
uint choose(uint16_t1x3 value){return 29u;}
uint choose(uint16_t1x4 value){return 30u;}
uint choose(uint16_t2x1 value){return 31u;}
uint choose(uint16_t2x2 value){return 32u;}
uint choose(uint16_t2x3 value){return 33u;}
uint choose(uint16_t2x4 value){return 34u;}
uint choose(uint16_t3x1 value){return 35u;}
uint choose(uint16_t3x2 value){return 36u;}
uint choose(uint16_t3x3 value){return 37u;}
uint choose(uint16_t3x4 value){return 38u;}
uint choose(uint16_t4x1 value){return 39u;}
uint choose(uint16_t4x2 value){return 40u;}
uint choose(uint16_t4x3 value){return 41u;}
uint choose(uint16_t4x4 value){return 42u;}
RWStructuredBuffer<uint> output;
[shader("compute")][numthreads(1,1,1)]void main()
{
    output[0]=choose(int16_t(0));
    output[1]=choose(int16_t1(0));
    output[2]=choose(int16_t2(0));
    output[3]=choose(int16_t3(0));
    output[4]=choose(int16_t4(0));
    output[5]=choose(int16_t1x1(0));
    output[6]=choose(int16_t1x2(0));
    output[7]=choose(int16_t1x3(0));
    output[8]=choose(int16_t1x4(0));
    output[9]=choose(int16_t2x1(0));
    output[10]=choose(int16_t2x2(0));
    output[11]=choose(int16_t2x3(0));
    output[12]=choose(int16_t2x4(0));
    output[13]=choose(int16_t3x1(0));
    output[14]=choose(int16_t3x2(0));
    output[15]=choose(int16_t3x3(0));
    output[16]=choose(int16_t3x4(0));
    output[17]=choose(int16_t4x1(0));
    output[18]=choose(int16_t4x2(0));
    output[19]=choose(int16_t4x3(0));
    output[20]=choose(int16_t4x4(0));
    output[21]=choose(uint16_t(0));
    output[22]=choose(uint16_t1(0));
    output[23]=choose(uint16_t2(0));
    output[24]=choose(uint16_t3(0));
    output[25]=choose(uint16_t4(0));
    output[26]=choose(uint16_t1x1(0));
    output[27]=choose(uint16_t1x2(0));
    output[28]=choose(uint16_t1x3(0));
    output[29]=choose(uint16_t1x4(0));
    output[30]=choose(uint16_t2x1(0));
    output[31]=choose(uint16_t2x2(0));
    output[32]=choose(uint16_t2x3(0));
    output[33]=choose(uint16_t2x4(0));
    output[34]=choose(uint16_t3x1(0));
    output[35]=choose(uint16_t3x2(0));
    output[36]=choose(uint16_t3x3(0));
    output[37]=choose(uint16_t3x4(0));
    output[38]=choose(uint16_t4x1(0));
    output[39]=choose(uint16_t4x2(0));
    output[40]=choose(uint16_t4x3(0));
    output[41]=choose(uint16_t4x4(0));
}
