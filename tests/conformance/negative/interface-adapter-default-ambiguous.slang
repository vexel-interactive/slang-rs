interface I{int read(float x);}struct V:I{int read(int x){return 11;}int read(int x,int y=7){return 22;}};
[shader("compute")][numthreads(1,1,1)]void main(){}
