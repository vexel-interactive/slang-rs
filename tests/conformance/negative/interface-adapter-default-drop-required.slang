interface I{int read(float x,int y);}struct V:I{int read(int x){return 11;}};
[shader("compute")][numthreads(1,1,1)]void main(){}
