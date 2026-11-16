interface I{int read(float x);}struct V:I{int read(int x,out int y=7){y=x;return y;}};
[shader("compute")][numthreads(1,1,1)]void main(){}
