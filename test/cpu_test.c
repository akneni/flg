#include <stdio.h>
#include <stdlib.h>
#include <math.h>
#include <unistd.h>
#include <fcntl.h>
#include <unistd.h>
#include <fcntl.h>

// Simulate CPU-intensive work with various call depths
volatile double sink;

double compute_inner(int iterations) {
    double result = 0.0;
    for (int i = 0; i < iterations; i++) {
        result += sin(i * 0.001) * cos(i * 0.002);
    }
    return result;
}

double compute_middle_a(int iterations) {
    double result = 0.0;
    for (int i = 0; i < iterations / 2; i++) {
        result += compute_inner(100);
    }
    return result;
}

double compute_middle_b(int iterations) {
    double result = 0.0;
    for (int i = 0; i < iterations / 3; i++) {
        result += compute_inner(150);
        result += sqrt(result * result + i);
    }
    return result;
}

double compute_outer_1(int iterations) {
    double result = 0.0;
    result += compute_middle_a(iterations);
    result += compute_middle_b(iterations / 2);
    return result;
}

double compute_outer_2(int iterations) {
    double result = 0.0;
    for (int i = 0; i < iterations; i++) {
        result += compute_inner(50);
        if (i % 100 == 0) {
            result += compute_middle_a(10);
        }
    }
    return result;
}

void heavy_allocation_work(int count) {
    for (int i = 0; i < count; i++) {
        int *arr = malloc(sizeof(int) * 1000);
        for (int j = 0; j < 1000; j++) {
            arr[j] = j * i;
        }
        sink = arr[500];
        free(arr);
    }
}

void perform_io_work(int count) {
    // Create a temporary file
    char filename[] = "temp_io_test_XXXXXX";
    int fd = mkstemp(filename);
    if (fd == -1) {
        perror("mkstemp");
        return;
    }

    // Unlink immediately so it's deleted on close
    unlink(filename);

    const int buf_size = 4096;
    char *buf = malloc(buf_size);
    // Fill buffer with random-ish data
    for (int i = 0; i < buf_size; i++) {
        buf[i] = (char)(i % 256);
    }

    // Do mixed read/write
    for (int i = 0; i < count; i++) {
        // Write
        if (lseek(fd, 0, SEEK_SET) == -1) break;
        if (write(fd, buf, buf_size) == -1) break;
        
        // Sync to force I/O
        fsync(fd);
        
        // Read back
        if (lseek(fd, 0, SEEK_SET) == -1) break;
        if (read(fd, buf, buf_size) == -1) break;
    }

    free(buf);
    close(fd);
}

int main(int argc, char *argv[]) {
    int iterations = 1;
    if (argc > 1) {
        iterations = atoi(argv[1]);
    }

    printf("Running CPU-intensive test with %d iterations...\n", iterations);

    double result = 0.0;
    
    // Multiple code paths for interesting flamegraph
    result += compute_outer_1(iterations);
    result += compute_outer_2(iterations);
    result += compute_outer_2(iterations);
    heavy_allocation_work(iterations * 10);
    perform_io_work(iterations / 20);
    perform_io_work(iterations / 20);
    
    printf("Result: %f\n", result);
    return 0;
}
