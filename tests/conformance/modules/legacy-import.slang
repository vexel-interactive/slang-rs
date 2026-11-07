import legacy;float run(){LegacyPair p;p.value=2;return legacyRead(p);}
RWStructuredBuffer<float> result;
[shader("compute")][numthreads(1,1,1)]void main(){result[0]=run();}
