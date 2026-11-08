// Ordered equal-width integer promotion across scalar, vector and matrix operands.
uint scalarKind(intptr_t x){return 5;}
uint vectorKind(vector<intptr_t,2> x){return 5;}
uint matrixKind(matrix<intptr_t,2,2> x){return 5;}
uint scalarKind(uintptr_t x){return 6;}
uint vectorKind(vector<uintptr_t,2> x){return 6;}
uint matrixKind(matrix<uintptr_t,2,2> x){return 6;}
uint scalarKind(int64_t x){return 3;}
uint vectorKind(vector<int64_t,2> x){return 3;}
uint matrixKind(matrix<int64_t,2,2> x){return 3;}
uint scalarKind(uint64_t x){return 4;}
uint vectorKind(vector<uint64_t,2> x){return 4;}
uint matrixKind(matrix<uint64_t,2,2> x){return 4;}

RWStructuredBuffer<uint> output;
[shader("compute")][numthreads(1,1,1)]void main()
{
    intptr_t s0=intptr_t(-9);vector<intptr_t,2> v0=vector<intptr_t,2>(s0,s0);matrix<intptr_t,2,2> m0=matrix<intptr_t,2,2>(s0,s0,s0,s0);
    uintptr_t s1=uintptr_t(2);vector<uintptr_t,2> v1=vector<uintptr_t,2>(s1,s1);matrix<uintptr_t,2,2> m1=matrix<uintptr_t,2,2>(s1,s1,s1,s1);
    int64_t s2=int64_t(-9);vector<int64_t,2> v2=vector<int64_t,2>(s2,s2);matrix<int64_t,2,2> m2=matrix<int64_t,2,2>(s2,s2,s2,s2);
    uint64_t s3=uint64_t(2);vector<uint64_t,2> v3=vector<uint64_t,2>(s3,s3);matrix<uint64_t,2,2> m3=matrix<uint64_t,2,2>(s3,s3,s3,s3);
    output[0]=scalarKind(s0+s0);
    output[1]=vectorKind(v0+s0);
    output[2]=vectorKind(s0+v0);
    output[3]=matrixKind(m0+s0);
    output[4]=matrixKind(s0+m0);
    output[5]=scalarKind(s0+s1);
    output[6]=vectorKind(v0+s1);
    output[7]=vectorKind(s0+v1);
    output[8]=matrixKind(m0+s1);
    output[9]=matrixKind(s0+m1);
    output[10]=scalarKind(s0+s2);
    output[11]=vectorKind(v0+s2);
    output[12]=vectorKind(s0+v2);
    output[13]=matrixKind(m0+s2);
    output[14]=matrixKind(s0+m2);
    output[15]=scalarKind(s0+s3);
    output[16]=vectorKind(v0+s3);
    output[17]=vectorKind(s0+v3);
    output[18]=matrixKind(m0+s3);
    output[19]=matrixKind(s0+m3);
    output[20]=scalarKind(s1+s0);
    output[21]=vectorKind(v1+s0);
    output[22]=vectorKind(s1+v0);
    output[23]=matrixKind(m1+s0);
    output[24]=matrixKind(s1+m0);
    output[25]=scalarKind(s1+s1);
    output[26]=vectorKind(v1+s1);
    output[27]=vectorKind(s1+v1);
    output[28]=matrixKind(m1+s1);
    output[29]=matrixKind(s1+m1);
    output[30]=scalarKind(s1+s2);
    output[31]=vectorKind(v1+s2);
    output[32]=vectorKind(s1+v2);
    output[33]=matrixKind(m1+s2);
    output[34]=matrixKind(s1+m2);
    output[35]=scalarKind(s1+s3);
    output[36]=vectorKind(v1+s3);
    output[37]=vectorKind(s1+v3);
    output[38]=matrixKind(m1+s3);
    output[39]=matrixKind(s1+m3);
    output[40]=scalarKind(s2+s0);
    output[41]=vectorKind(v2+s0);
    output[42]=vectorKind(s2+v0);
    output[43]=matrixKind(m2+s0);
    output[44]=matrixKind(s2+m0);
    output[45]=scalarKind(s2+s1);
    output[46]=vectorKind(v2+s1);
    output[47]=vectorKind(s2+v1);
    output[48]=matrixKind(m2+s1);
    output[49]=matrixKind(s2+m1);
    output[50]=scalarKind(s2+s2);
    output[51]=vectorKind(v2+s2);
    output[52]=vectorKind(s2+v2);
    output[53]=matrixKind(m2+s2);
    output[54]=matrixKind(s2+m2);
    output[55]=scalarKind(s2+s3);
    output[56]=vectorKind(v2+s3);
    output[57]=vectorKind(s2+v3);
    output[58]=matrixKind(m2+s3);
    output[59]=matrixKind(s2+m3);
    output[60]=scalarKind(s3+s0);
    output[61]=vectorKind(v3+s0);
    output[62]=vectorKind(s3+v0);
    output[63]=matrixKind(m3+s0);
    output[64]=matrixKind(s3+m0);
    output[65]=scalarKind(s3+s1);
    output[66]=vectorKind(v3+s1);
    output[67]=vectorKind(s3+v1);
    output[68]=matrixKind(m3+s1);
    output[69]=matrixKind(s3+m1);
    output[70]=scalarKind(s3+s2);
    output[71]=vectorKind(v3+s2);
    output[72]=vectorKind(s3+v2);
    output[73]=matrixKind(m3+s2);
    output[74]=matrixKind(s3+m2);
    output[75]=scalarKind(s3+s3);
    output[76]=vectorKind(v3+s3);
    output[77]=vectorKind(s3+v3);
    output[78]=matrixKind(m3+s3);
    output[79]=matrixKind(s3+m3);
}
