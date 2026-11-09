struct Box<T>{T value;int read(){if(value)return 1;return 0;}};[shader("compute")][numthreads(1,1,1)]void main(){}
