include_guard(GLOBAL)

include(FetchContent)

FetchContent_Declare(
    googletest
    URL https://github.com/google/googletest/releases/download/v1.17.0/googletest-1.17.0.tar.gz
    DOWNLOAD_EXTRACT_TIMESTAMP TRUE
)
FetchContent_MakeAvailable(googletest)

FetchContent_Declare(
    abseil
    URL https://github.com/abseil/abseil-cpp/releases/download/20250814.1/abseil-cpp-20250814.1.tar.gz
    DOWNLOAD_EXTRACT_TIMESTAMP TRUE
)
FetchContent_MakeAvailable(abseil)

set(BENCHMARK_ENABLE_GTEST_TESTS OFF)
FetchContent_Declare(
    benchmark
    URL https://github.com/google/benchmark/archive/refs/tags/v1.9.4.tar.gz
    DOWNLOAD_EXTRACT_TIMESTAMP TRUE
)
FetchContent_MakeAvailable(benchmark)

FetchContent_Declare(
    ethash
    URL https://github.com/chfast/ethash/archive/refs/tags/v1.1.0.tar.gz
    DOWNLOAD_EXTRACT_TIMESTAMP TRUE
)
FetchContent_MakeAvailable(ethash)

set(TRACY_STATIC OFF)
FetchContent_Declare(
    tracy
    URL https://github.com/wolfpld/tracy/archive/refs/tags/v0.9.1.tar.gz
    DOWNLOAD_EXTRACT_TIMESTAMP TRUE
)
FetchContent_MakeAvailable(tracy)

FetchContent_Declare(
    intx
    URL https://github.com/chfast/intx/archive/refs/tags/v0.13.0.tar.gz
    DOWNLOAD_EXTRACT_TIMESTAMP TRUE
)
FetchContent_MakeAvailable(intx)


if (NOT ${CMAKE_SYSTEM_NAME} MATCHES "Darwin")
    set(BUILD_TESTING OFF)
    FetchContent_Declare(
        gperftools
        URL https://github.com/gperftools/gperftools/archive/refs/tags/gperftools-2.17.2.tar.gz
        DOWNLOAD_EXTRACT_TIMESTAMP TRUE
        )
    FetchContent_MakeAvailable(gperftools)
endif()


set(MI_OVERRIDE OFF)
set(MI_BUILD_TESTS OFF)
FetchContent_Declare(
    mimalloc
    URL https://github.com/microsoft/mimalloc/archive/refs/tags/v3.0.10.tar.gz
    DOWNLOAD_EXTRACT_TIMESTAMP TRUE
)
FetchContent_MakeAvailable(mimalloc)