// Arithmetic joining follows ordered component sizes in the pinned core.
uint choose(int16_t value){return 1u;}
uint choose(uint16_t value){return 2u;}
uint choose(int value){return 3u;}
uint choose(uint value){return 4u;}
uint choose(int64_t value){return 5u;}
uint choose(uint64_t value){return 6u;}
uint choose(intptr_t value){return 7u;}
uint choose(uintptr_t value){return 8u;}
uint choose(half value){return 9u;}
uint choose(float value){return 10u;}
uint choose(double value){return 11u;}
RWStructuredBuffer<uint> output;
[shader("compute")][numthreads(1,1,1)]void main()
{
    output[0]=choose(int16_t(0)+int16_t(0));
    output[1]=choose(int16_t(0)+uint16_t(0));
    output[2]=choose(int16_t(0)+int(0));
    output[3]=choose(int16_t(0)+uint(0));
    output[4]=choose(int16_t(0)+int64_t(0));
    output[5]=choose(int16_t(0)+uint64_t(0));
    output[6]=choose(int16_t(0)+intptr_t(0));
    output[7]=choose(int16_t(0)+uintptr_t(0));
    output[8]=choose(int16_t(0)+half(0));
    output[9]=choose(int16_t(0)+float(0));
    output[10]=choose(int16_t(0)+double(0));
    output[11]=choose(uint16_t(0)+int16_t(0));
    output[12]=choose(uint16_t(0)+uint16_t(0));
    output[13]=choose(uint16_t(0)+int(0));
    output[14]=choose(uint16_t(0)+uint(0));
    output[15]=choose(uint16_t(0)+int64_t(0));
    output[16]=choose(uint16_t(0)+uint64_t(0));
    output[17]=choose(uint16_t(0)+intptr_t(0));
    output[18]=choose(uint16_t(0)+uintptr_t(0));
    output[19]=choose(uint16_t(0)+half(0));
    output[20]=choose(uint16_t(0)+float(0));
    output[21]=choose(uint16_t(0)+double(0));
    output[22]=choose(int(0)+int16_t(0));
    output[23]=choose(int(0)+uint16_t(0));
    output[24]=choose(int(0)+int(0));
    output[25]=choose(int(0)+uint(0));
    output[26]=choose(int(0)+int64_t(0));
    output[27]=choose(int(0)+uint64_t(0));
    output[28]=choose(int(0)+intptr_t(0));
    output[29]=choose(int(0)+uintptr_t(0));
    output[30]=choose(int(0)+half(0));
    output[31]=choose(int(0)+float(0));
    output[32]=choose(int(0)+double(0));
    output[33]=choose(uint(0)+int16_t(0));
    output[34]=choose(uint(0)+uint16_t(0));
    output[35]=choose(uint(0)+int(0));
    output[36]=choose(uint(0)+uint(0));
    output[37]=choose(uint(0)+int64_t(0));
    output[38]=choose(uint(0)+uint64_t(0));
    output[39]=choose(uint(0)+intptr_t(0));
    output[40]=choose(uint(0)+uintptr_t(0));
    output[41]=choose(uint(0)+half(0));
    output[42]=choose(uint(0)+float(0));
    output[43]=choose(uint(0)+double(0));
    output[44]=choose(int64_t(0)+int16_t(0));
    output[45]=choose(int64_t(0)+uint16_t(0));
    output[46]=choose(int64_t(0)+int(0));
    output[47]=choose(int64_t(0)+uint(0));
    output[48]=choose(int64_t(0)+int64_t(0));
    output[49]=choose(int64_t(0)+uint64_t(0));
    output[50]=choose(int64_t(0)+intptr_t(0));
    output[51]=choose(int64_t(0)+uintptr_t(0));
    output[52]=choose(int64_t(0)+half(0));
    output[53]=choose(int64_t(0)+float(0));
    output[54]=choose(int64_t(0)+double(0));
    output[55]=choose(uint64_t(0)+int16_t(0));
    output[56]=choose(uint64_t(0)+uint16_t(0));
    output[57]=choose(uint64_t(0)+int(0));
    output[58]=choose(uint64_t(0)+uint(0));
    output[59]=choose(uint64_t(0)+int64_t(0));
    output[60]=choose(uint64_t(0)+uint64_t(0));
    output[61]=choose(uint64_t(0)+intptr_t(0));
    output[62]=choose(uint64_t(0)+uintptr_t(0));
    output[63]=choose(uint64_t(0)+half(0));
    output[64]=choose(uint64_t(0)+float(0));
    output[65]=choose(uint64_t(0)+double(0));
    output[66]=choose(intptr_t(0)+int16_t(0));
    output[67]=choose(intptr_t(0)+uint16_t(0));
    output[68]=choose(intptr_t(0)+int(0));
    output[69]=choose(intptr_t(0)+uint(0));
    output[70]=choose(intptr_t(0)+int64_t(0));
    output[71]=choose(intptr_t(0)+uint64_t(0));
    output[72]=choose(intptr_t(0)+intptr_t(0));
    output[73]=choose(intptr_t(0)+uintptr_t(0));
    output[74]=choose(intptr_t(0)+half(0));
    output[75]=choose(intptr_t(0)+float(0));
    output[76]=choose(intptr_t(0)+double(0));
    output[77]=choose(uintptr_t(0)+int16_t(0));
    output[78]=choose(uintptr_t(0)+uint16_t(0));
    output[79]=choose(uintptr_t(0)+int(0));
    output[80]=choose(uintptr_t(0)+uint(0));
    output[81]=choose(uintptr_t(0)+int64_t(0));
    output[82]=choose(uintptr_t(0)+uint64_t(0));
    output[83]=choose(uintptr_t(0)+intptr_t(0));
    output[84]=choose(uintptr_t(0)+uintptr_t(0));
    output[85]=choose(uintptr_t(0)+half(0));
    output[86]=choose(uintptr_t(0)+float(0));
    output[87]=choose(uintptr_t(0)+double(0));
    output[88]=choose(half(0)+int16_t(0));
    output[89]=choose(half(0)+uint16_t(0));
    output[90]=choose(half(0)+int(0));
    output[91]=choose(half(0)+uint(0));
    output[92]=choose(half(0)+int64_t(0));
    output[93]=choose(half(0)+uint64_t(0));
    output[94]=choose(half(0)+intptr_t(0));
    output[95]=choose(half(0)+uintptr_t(0));
    output[96]=choose(half(0)+half(0));
    output[97]=choose(half(0)+float(0));
    output[98]=choose(half(0)+double(0));
    output[99]=choose(float(0)+int16_t(0));
    output[100]=choose(float(0)+uint16_t(0));
    output[101]=choose(float(0)+int(0));
    output[102]=choose(float(0)+uint(0));
    output[103]=choose(float(0)+int64_t(0));
    output[104]=choose(float(0)+uint64_t(0));
    output[105]=choose(float(0)+intptr_t(0));
    output[106]=choose(float(0)+uintptr_t(0));
    output[107]=choose(float(0)+half(0));
    output[108]=choose(float(0)+float(0));
    output[109]=choose(float(0)+double(0));
    output[110]=choose(double(0)+int16_t(0));
    output[111]=choose(double(0)+uint16_t(0));
    output[112]=choose(double(0)+int(0));
    output[113]=choose(double(0)+uint(0));
    output[114]=choose(double(0)+int64_t(0));
    output[115]=choose(double(0)+uint64_t(0));
    output[116]=choose(double(0)+intptr_t(0));
    output[117]=choose(double(0)+uintptr_t(0));
    output[118]=choose(double(0)+half(0));
    output[119]=choose(double(0)+float(0));
    output[120]=choose(double(0)+double(0));
}
