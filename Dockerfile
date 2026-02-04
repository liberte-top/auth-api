# Build stage
FROM maven:3.9.8-eclipse-temurin-21 AS builder
WORKDIR /app
COPY pom.xml ./
RUN mvn -q -DskipTests dependency:go-offline
COPY src ./src
RUN mvn -q -DskipTests package

# Runtime stage
FROM eclipse-temurin:21-jre-jammy
WORKDIR /app
COPY --from=builder /app/target/auth-api-0.1.0.jar /app/auth-api.jar

ENV JAVA_OPTS=""
EXPOSE 3333
CMD ["sh", "-c", "java $JAVA_OPTS -jar /app/auth-api.jar"]
