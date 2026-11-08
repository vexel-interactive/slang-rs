// Pointer literal boundaries from the pinned frontend, plus suffix and prefix contexts.
uint getType(int x){return 1;}
uint getType(uint x){return 2;}
uint getType(intptr_t x){return 5;}
uint getType(uintptr_t x){return 6;}
uint getType(int64_t x){return 3;}
uint getType(uint64_t x){return 4;}

RWStructuredBuffer<uint> output;
[shader("compute")][numthreads(1,1,1)]void main()
{
    output[0]=getType(0Z);
    output[1]=getType(9223372036854775807Z);
    output[2]=getType(0x0Z);
    output[3]=getType(0xFFFFFFFFFFFFFFFFZ);
    output[4]=getType(0UZ);
    output[5]=getType(18446744073709551615UZ);
    output[6]=getType(0ZU);
    output[7]=getType(18446744073709551615ZU);
    output[8]=getType(0x0UZ);
    output[9]=getType(0xFFFFFFFFFFFFFFFFUZ);
    output[10]=getType(-9223372036854775808Z);
    output[11]=getType(2147483648z);
    output[12]=getType(9223372036854775808z);
    output[13]=getType(18446744073709551615z);
    output[14]=getType(-2147483648z);
    output[15]=getType(-9223372036854775808z);
    output[16]=getType(0x8000000000000000z);
    output[17]=getType(0b1111111111111111111111111111111111111111111111111111111111111111z);
    output[18]=getType(01777777777777777777777z);
    output[19]=getType(2147483648Z);
    output[20]=getType(9223372036854775808Z);
    output[21]=getType(18446744073709551615Z);
    output[22]=getType(-2147483648Z);
    output[23]=getType(-9223372036854775808Z);
    output[24]=getType(0x8000000000000000Z);
    output[25]=getType(0b1111111111111111111111111111111111111111111111111111111111111111Z);
    output[26]=getType(01777777777777777777777Z);
    output[27]=getType(1uz);
    output[28]=getType(0xffffffffffffffffuz);
    output[29]=getType(1UZ);
    output[30]=getType(0xffffffffffffffffUZ);
    output[31]=getType(1zu);
    output[32]=getType(0xffffffffffffffffzu);
    output[33]=getType(1ZU);
    output[34]=getType(0xffffffffffffffffZU);
    output[35]=getType(1Uz);
    output[36]=getType(0xffffffffffffffffUz);
    output[37]=getType(1uZ);
    output[38]=getType(0xffffffffffffffffuZ);
    output[39]=getType(1Zu);
    output[40]=getType(0xffffffffffffffffZu);
    output[41]=getType(1zU);
    output[42]=getType(0xffffffffffffffffzU);
    output[43]=getType(+(9223372036854775808Z));
    output[44]=getType(-(9223372036854775808Z));
    output[45]=getType(+-9223372036854775808Z);
    output[46]=getType(-+9223372036854775808Z);
    output[47]=getType(~0xffffffffffffffffZ);
    output[48]=getType(-0xffffffffffffffffZ);
    output[49]=getType(0x1315312213712224Z);
    output[50]=getType(0x7f3961749ac87288UZ);
}
