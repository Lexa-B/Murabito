#pragma once

#include "CoreMinimal.h"
#include "GameFramework/GameModeBase.h"
#include "GameRules.generated.h"

/**
 * The project's game mode. Uses the overhead camera rig and its input controller by default;
 * the Blueprint child (BP_GameRules) swaps in their Blueprint children, which hold the input
 * assets and tuning values.
 */
UCLASS(Blueprintable)
class MURABITO_API AGameRules : public AGameModeBase
{
	GENERATED_BODY()

public:
	AGameRules();

protected:
	virtual void BeginPlay() override;
};
