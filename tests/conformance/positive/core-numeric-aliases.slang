// All 105 supported scalar, vector and matrix aliases from the pinned core type table.
uint kind(float16_t value){return 1;}
uint kind(float16_t1 value){return 2;}
uint kind(float16_t2 value){return 3;}
uint kind(float16_t3 value){return 4;}
uint kind(float16_t4 value){return 5;}
uint kind(float16_t1x1 value){return 6;}
uint kind(float16_t1x2 value){return 7;}
uint kind(float16_t1x3 value){return 8;}
uint kind(float16_t1x4 value){return 9;}
uint kind(float16_t2x1 value){return 10;}
uint kind(float16_t2x2 value){return 11;}
uint kind(float16_t2x3 value){return 12;}
uint kind(float16_t2x4 value){return 13;}
uint kind(float16_t3x1 value){return 14;}
uint kind(float16_t3x2 value){return 15;}
uint kind(float16_t3x3 value){return 16;}
uint kind(float16_t3x4 value){return 17;}
uint kind(float16_t4x1 value){return 18;}
uint kind(float16_t4x2 value){return 19;}
uint kind(float16_t4x3 value){return 20;}
uint kind(float16_t4x4 value){return 21;}
uint kind(float32_t value){return 22;}
uint kind(float32_t1 value){return 23;}
uint kind(float32_t2 value){return 24;}
uint kind(float32_t3 value){return 25;}
uint kind(float32_t4 value){return 26;}
uint kind(float32_t1x1 value){return 27;}
uint kind(float32_t1x2 value){return 28;}
uint kind(float32_t1x3 value){return 29;}
uint kind(float32_t1x4 value){return 30;}
uint kind(float32_t2x1 value){return 31;}
uint kind(float32_t2x2 value){return 32;}
uint kind(float32_t2x3 value){return 33;}
uint kind(float32_t2x4 value){return 34;}
uint kind(float32_t3x1 value){return 35;}
uint kind(float32_t3x2 value){return 36;}
uint kind(float32_t3x3 value){return 37;}
uint kind(float32_t3x4 value){return 38;}
uint kind(float32_t4x1 value){return 39;}
uint kind(float32_t4x2 value){return 40;}
uint kind(float32_t4x3 value){return 41;}
uint kind(float32_t4x4 value){return 42;}
uint kind(float64_t value){return 43;}
uint kind(float64_t1 value){return 44;}
uint kind(float64_t2 value){return 45;}
uint kind(float64_t3 value){return 46;}
uint kind(float64_t4 value){return 47;}
uint kind(float64_t1x1 value){return 48;}
uint kind(float64_t1x2 value){return 49;}
uint kind(float64_t1x3 value){return 50;}
uint kind(float64_t1x4 value){return 51;}
uint kind(float64_t2x1 value){return 52;}
uint kind(float64_t2x2 value){return 53;}
uint kind(float64_t2x3 value){return 54;}
uint kind(float64_t2x4 value){return 55;}
uint kind(float64_t3x1 value){return 56;}
uint kind(float64_t3x2 value){return 57;}
uint kind(float64_t3x3 value){return 58;}
uint kind(float64_t3x4 value){return 59;}
uint kind(float64_t4x1 value){return 60;}
uint kind(float64_t4x2 value){return 61;}
uint kind(float64_t4x3 value){return 62;}
uint kind(float64_t4x4 value){return 63;}
uint kind(int32_t value){return 64;}
uint kind(int32_t1 value){return 65;}
uint kind(int32_t2 value){return 66;}
uint kind(int32_t3 value){return 67;}
uint kind(int32_t4 value){return 68;}
uint kind(int32_t1x1 value){return 69;}
uint kind(int32_t1x2 value){return 70;}
uint kind(int32_t1x3 value){return 71;}
uint kind(int32_t1x4 value){return 72;}
uint kind(int32_t2x1 value){return 73;}
uint kind(int32_t2x2 value){return 74;}
uint kind(int32_t2x3 value){return 75;}
uint kind(int32_t2x4 value){return 76;}
uint kind(int32_t3x1 value){return 77;}
uint kind(int32_t3x2 value){return 78;}
uint kind(int32_t3x3 value){return 79;}
uint kind(int32_t3x4 value){return 80;}
uint kind(int32_t4x1 value){return 81;}
uint kind(int32_t4x2 value){return 82;}
uint kind(int32_t4x3 value){return 83;}
uint kind(int32_t4x4 value){return 84;}
uint kind(uint32_t value){return 85;}
uint kind(uint32_t1 value){return 86;}
uint kind(uint32_t2 value){return 87;}
uint kind(uint32_t3 value){return 88;}
uint kind(uint32_t4 value){return 89;}
uint kind(uint32_t1x1 value){return 90;}
uint kind(uint32_t1x2 value){return 91;}
uint kind(uint32_t1x3 value){return 92;}
uint kind(uint32_t1x4 value){return 93;}
uint kind(uint32_t2x1 value){return 94;}
uint kind(uint32_t2x2 value){return 95;}
uint kind(uint32_t2x3 value){return 96;}
uint kind(uint32_t2x4 value){return 97;}
uint kind(uint32_t3x1 value){return 98;}
uint kind(uint32_t3x2 value){return 99;}
uint kind(uint32_t3x3 value){return 100;}
uint kind(uint32_t3x4 value){return 101;}
uint kind(uint32_t4x1 value){return 102;}
uint kind(uint32_t4x2 value){return 103;}
uint kind(uint32_t4x3 value){return 104;}
uint kind(uint32_t4x4 value){return 105;}
RWStructuredBuffer<uint> output;
[shader("compute")][numthreads(1,1,1)]void main(){
    output[0]=kind(float16_t(1));
    output[1]=kind(float16_t1(1));
    output[2]=kind(float16_t2(1));
    output[3]=kind(float16_t3(1));
    output[4]=kind(float16_t4(1));
    output[5]=kind(float16_t1x1(1));
    output[6]=kind(float16_t1x2(1));
    output[7]=kind(float16_t1x3(1));
    output[8]=kind(float16_t1x4(1));
    output[9]=kind(float16_t2x1(1));
    output[10]=kind(float16_t2x2(1));
    output[11]=kind(float16_t2x3(1));
    output[12]=kind(float16_t2x4(1));
    output[13]=kind(float16_t3x1(1));
    output[14]=kind(float16_t3x2(1));
    output[15]=kind(float16_t3x3(1));
    output[16]=kind(float16_t3x4(1));
    output[17]=kind(float16_t4x1(1));
    output[18]=kind(float16_t4x2(1));
    output[19]=kind(float16_t4x3(1));
    output[20]=kind(float16_t4x4(1));
    output[21]=kind(float32_t(1));
    output[22]=kind(float32_t1(1));
    output[23]=kind(float32_t2(1));
    output[24]=kind(float32_t3(1));
    output[25]=kind(float32_t4(1));
    output[26]=kind(float32_t1x1(1));
    output[27]=kind(float32_t1x2(1));
    output[28]=kind(float32_t1x3(1));
    output[29]=kind(float32_t1x4(1));
    output[30]=kind(float32_t2x1(1));
    output[31]=kind(float32_t2x2(1));
    output[32]=kind(float32_t2x3(1));
    output[33]=kind(float32_t2x4(1));
    output[34]=kind(float32_t3x1(1));
    output[35]=kind(float32_t3x2(1));
    output[36]=kind(float32_t3x3(1));
    output[37]=kind(float32_t3x4(1));
    output[38]=kind(float32_t4x1(1));
    output[39]=kind(float32_t4x2(1));
    output[40]=kind(float32_t4x3(1));
    output[41]=kind(float32_t4x4(1));
    output[42]=kind(float64_t(1));
    output[43]=kind(float64_t1(1));
    output[44]=kind(float64_t2(1));
    output[45]=kind(float64_t3(1));
    output[46]=kind(float64_t4(1));
    output[47]=kind(float64_t1x1(1));
    output[48]=kind(float64_t1x2(1));
    output[49]=kind(float64_t1x3(1));
    output[50]=kind(float64_t1x4(1));
    output[51]=kind(float64_t2x1(1));
    output[52]=kind(float64_t2x2(1));
    output[53]=kind(float64_t2x3(1));
    output[54]=kind(float64_t2x4(1));
    output[55]=kind(float64_t3x1(1));
    output[56]=kind(float64_t3x2(1));
    output[57]=kind(float64_t3x3(1));
    output[58]=kind(float64_t3x4(1));
    output[59]=kind(float64_t4x1(1));
    output[60]=kind(float64_t4x2(1));
    output[61]=kind(float64_t4x3(1));
    output[62]=kind(float64_t4x4(1));
    output[63]=kind(int32_t(1));
    output[64]=kind(int32_t1(1));
    output[65]=kind(int32_t2(1));
    output[66]=kind(int32_t3(1));
    output[67]=kind(int32_t4(1));
    output[68]=kind(int32_t1x1(1));
    output[69]=kind(int32_t1x2(1));
    output[70]=kind(int32_t1x3(1));
    output[71]=kind(int32_t1x4(1));
    output[72]=kind(int32_t2x1(1));
    output[73]=kind(int32_t2x2(1));
    output[74]=kind(int32_t2x3(1));
    output[75]=kind(int32_t2x4(1));
    output[76]=kind(int32_t3x1(1));
    output[77]=kind(int32_t3x2(1));
    output[78]=kind(int32_t3x3(1));
    output[79]=kind(int32_t3x4(1));
    output[80]=kind(int32_t4x1(1));
    output[81]=kind(int32_t4x2(1));
    output[82]=kind(int32_t4x3(1));
    output[83]=kind(int32_t4x4(1));
    output[84]=kind(uint32_t(1));
    output[85]=kind(uint32_t1(1));
    output[86]=kind(uint32_t2(1));
    output[87]=kind(uint32_t3(1));
    output[88]=kind(uint32_t4(1));
    output[89]=kind(uint32_t1x1(1));
    output[90]=kind(uint32_t1x2(1));
    output[91]=kind(uint32_t1x3(1));
    output[92]=kind(uint32_t1x4(1));
    output[93]=kind(uint32_t2x1(1));
    output[94]=kind(uint32_t2x2(1));
    output[95]=kind(uint32_t2x3(1));
    output[96]=kind(uint32_t2x4(1));
    output[97]=kind(uint32_t3x1(1));
    output[98]=kind(uint32_t3x2(1));
    output[99]=kind(uint32_t3x3(1));
    output[100]=kind(uint32_t3x4(1));
    output[101]=kind(uint32_t4x1(1));
    output[102]=kind(uint32_t4x2(1));
    output[103]=kind(uint32_t4x3(1));
    output[104]=kind(uint32_t4x4(1));
}
