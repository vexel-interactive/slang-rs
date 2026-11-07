import members;
#ifdef MEMBER_ONLY
#error member macro leaked
#endif
float run(){return memberRead();}
RWStructuredBuffer<float> result;
[shader("compute")][numthreads(1,1,1)]void main(){result[0]=run();}
