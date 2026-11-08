// Generic joins, literal conversion ranking, aliases and 32-bit reinterpretation overloads.
uint kind(intptr_t x){return 5;}uint kind(uintptr_t x){return 6;}uint kind(int64_t x){return 3;}uint kind(uint64_t x){return 4;}uint kind(float x){return 7;}uint kind(uint x){return 2;}uint kind(vector<uint,2> x){return 2;}uint kind(vector<intptr_t,2> x){return 5;}uint matrixKind(matrix<uint,2,2> x){return 2;}uint matrixKind(matrix<intptr_t,2,2> x){return 5;}
uint preferSigned(intptr_t x){return 1;}uint preferSigned(uint64_t x){return 2;}
uint preferUnsigned(intptr_t x){return 1;}uint preferUnsigned(float x){return 2;}
uint promotePointer(int64_t x){return 1;}uint promotePointer(uintptr_t x){return 2;}
uint convertUnsigned(int64_t x){return 1;}uint convertUnsigned(float x){return 2;}
RWStructuredBuffer<uint> output;
[shader("compute")][numthreads(1,1,1)]void main(){
    uint small=1u;vector<intptr_t,2> pair=3Z;matrix<intptr_t,2,2> matrixValue=3Z;intptr_t p=-9Z;uintptr_t u=2UZ;int64_t s=-7LL;uint64_t t=3ULL;
    output[0]=kind(true?p:s);
    output[1]=kind(true?s:p);
    output[2]=kind(true?u:t);
    output[3]=kind(true?t:u);
    output[4]=kind(min(p,s));
    output[5]=kind(min(s,p));
    output[6]=kind(min(u,t));
    output[7]=kind(min(t,u));
    output[8]=kind(max(p,s));
    output[9]=kind(clamp(p,s,p));
    output[10]=kind(mul(p,s));
    output[11]=preferSigned(1);
    output[12]=preferUnsigned(1u);
    output[13]=promotePointer(p);
    output[14]=convertUnsigned(u);
    output[15]=kind(ssize_t(-1));
    output[16]=kind(size_t(1));
    output[17]=kind(usize_t(1));
    output[18]=asuint(asfloat(1Z));
    output[19]=uint(asint(2UZ));
    output[20]=asuint(3Z);
    output[21]=preferUnsigned(small);
    output[22]=asuint(asfloat(p));
    output[23]=uint(asint(u));
    output[24]=asuint(p);
    output[25]=kind(dot(small,pair));
    output[26]=kind(dot(pair,small));
    output[27]=kind(min(small,pair));
    output[28]=kind(min(pair,small));
    output[29]=kind(mul(small,pair));
    output[30]=kind(mul(pair,small));
    output[31]=matrixKind(mul(small,matrixValue));
    output[32]=matrixKind(mul(matrixValue,small));
    output[33]=kind(true?small:pair);
    output[34]=kind(true?pair:small);
    output[35]=uint(max(small,p)/2u);
    output[36]=uint((true?small:p)/2u);
}
